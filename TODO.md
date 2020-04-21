## Done today

* Fixed pixie stitch to work with new shipping workflow
* Windows shipping build does not crash when resources tool is not available
* Adds installation instructions to pixie stitch 
* Adds template instructions for new projects
* Re-add old gamecode from previous bytepath implementation to our current project
* Reads display index and controller deadzone thresholds from config file
* Moved WINDOW_TITLE, SAVE_FOLDER_NAME and COMPANY_NAME to its own generated file 
* Pixie Stitch: 
  - Implement origin based coordinate marker system
  - Adds additional ouput folder for center based coordinate patterns
  - Showing of edge coordinate labels

## Next:

## Backlog:


* Pixie Stitch: 
  - Add custom launcher icon


* We need a production/develop version where we enable/disable i.e. panic messageboxes. It would be 
  useful to having a config file for this that is read on startup. Maybe this can be the same as the 
  display / controller config file? We want to configure/enable/disable the following things:
  - Show panics in messageboxes
  - Debug print frametimes
  - Set log levels
  - Splashscreen
* Clean up old stuff code at the end of draw.rs and sdl_window.rs. 
  Determine what is needed and implement it. Throw out the rest 
* Easy debug-printing text API that draws in screenspace (not canvas-space)
  - We need to add a debug layer to the drawstate with its own drawqueue

* Allow hotreloading of game assets
* Support ogg audio and differentiate between mono/stereo recordings

* Add modulators like in https://www.youtube.com/watch?v=n-txrCMvdms especially shift register 
  modulator and newtonian following modulator

* Update version info resource with the crate version
* Zip up final executable and add version from crate
* We need to certify our Windows executable with a real certificate
  Maybe like this one:
  https://codesigncert.com/cheapcodesigning
  Also useful:
  https://social.technet.microsoft.com/wiki/contents/articles/38117.microsoft-trusted-root-certificate-program-participants-as-of-june-27-2017.aspx#C

* Future tutorial games:
  - https://simplegametutorials.github.io/
  - https://github.com/noooway/love2d_arkanoid_tutorial
  - https://github.com/adnzzzzZ/blog/issues/30
  - https://inventwithpython.com/blog/2012/02/20/i-need-practice-programming-49-ideas-for-game-clones-to-code/
  - https://gamedev.stackexchange.com/a/945
  - https://www.gamedev.net/articles/programming/general-and-gameplay-programming/your-first-step-to-game-development-starts-here-r2976
  - https://bfnightly.bracketproductions.com/rustbook/chapter_0.html


