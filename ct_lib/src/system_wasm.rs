pub struct Fileloader {
    // NOTE: We save filepath here because sometimes `request.response_url()` gives us an empty
    //       string (i.e. when crashing while doing a Cross-Origin Requests (COR))
    filepath: String,
    request: web_sys::XmlHttpRequest,
    finished: bool,
}

impl Fileloader {
    pub fn new(filepath: &str) -> Result<Fileloader, String> {
        let request = web_sys::XmlHttpRequest::new().expect("Failed to make XmlHttpRequest");
        request.open("GET", filepath).map_err(|error| {
            format!(
                "Failed to create GET request for '{}' - {:?}",
                filepath, error
            )
        })?;

        request.set_response_type(web_sys::XmlHttpRequestResponseType::Arraybuffer);
        request.send().map_err(|error| {
            format!(
                "Failed to send GET request for '{}' - {:?}",
                filepath, error
            )
        })?;

        Ok(Fileloader {
            filepath: filepath.to_owned(),
            request,
            finished: false,
        })
    }

    pub fn is_done(&self) -> bool {
        self.request.ready_state() == web_sys::XmlHttpRequest::DONE
    }

    pub fn poll(&mut self) -> Result<Option<Vec<u8>>, String> {
        assert!(!self.finished);

        match self.request.ready_state() {
            web_sys::XmlHttpRequest::DONE => {
                let status = self.request.status().map_err(|error| {
                    format!(
                        "Failed to get request status for '{}' - {:?}",
                        &self.filepath, error
                    )
                })?;

                if status / 100 == 2 {
                    // Success (statuscode 2xx)
                    self.finished = true;
                    let response = self.request.response().map_err(|error| {
                        format!(
                            "Failed to read response for '{}' - {:?}",
                            &self.filepath, error
                        )
                    })?;
                    let array = js_sys::Uint8Array::new(&response);
                    let mut result = vec![0u8; array.length() as usize];
                    array.copy_to(&mut result);
                    Ok(Some(result))
                } else {
                    // Failed (statuscode != 2xx)
                    self.finished = true;
                    let status_text = self.request.status_text().unwrap_or("Unknown".to_owned());
                    Err(format!(
                        "Failed to load file '{}' - Status: {} - {:?}",
                        &self.filepath, status, status_text
                    ))
                }
            }
            _ => Ok(None),
        }
    }
}

pub fn path_join(first: &str, second: &str) -> String {
    if first.ends_with('/') || first.ends_with('\\') {
        format!("{}{}", first, second).replace("\\", "/")
    } else {
        format!("{}/{}", first, second).replace("\\", "/")
    }
}
