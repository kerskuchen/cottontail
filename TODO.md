# DONE:

- let audiosources resample their output

# CURRENT


# NEXT:



## improve examples
- find a good way to switch/disable audio when switching scenes
- restore gui/credits displaying as an example scene
- convert debug scene to example in a dedicated examples folder with its own assets and build scripts

## writing games easier
- make draw/audio/other things global for easier use (we run everything on the same thread anyway)
- Easier text drawing api
  - one that is simple without much parameters
  - one that just centers text in rect
  - macro based debug log with format strings?
  - other ideas?
- create a internal mode feature flag and
  - to use local folder for logging
  - enable debug draw logging
- Ability to draw debug graphs to i.e. try out attenuation for audio distance
  - We need to add a debug layer to the drawstate with its own drawqueue
- add unified/virtual gamecursor input to gamecursors struct (uses mouse or first touchfinger)
- simplify touch input query (especially within the appcursors context)
- Change linestrip drawing api to take a `loop` parameter so we can get rid of 5 vertex 
  sized rectangle drawing and the `skip_last_vertex` 
- Fix Vec2 to work with flipped_y only and remove special suffixes?
- Add modulators like in https://www.youtube.com/watch?v=n-txrCMvdms especially shift register 
  modulator and newtonian following modulator
- replace math::Interval by Rust range with trait methods
- Future tutorial games:
  - https://simplegametutorials.github.io/
  - https://github.com/noooway/love2d_arkanoid_tutorial
  - https://github.com/adnzzzzZ/blog/issues/30
  - https://inventwithpython.com/blog/2012/02/20/i-need-practice-programming-49-ideas-for-game-clones-to-code/
  - https://gamedev.stackexchange.com/a/945
  - https://www.gamedev.net/articles/programming/general-and-gameplay-programming/your-first-step-to-game-development-starts-here-r2976
  - https://bfnightly.bracketproductions.com/rustbook/chapter_0.html

## better platform layer
- implement appcommands in wasm
- make screen orientation settable
- rename things that are not necessarily game related to app
- also do we need resize callbacks at all? (also in sdl2)?
- fix mouseup/touchup events that happen outside of browser window (i.e. affects leaving fullscreen)
  we may need https://developer.mozilla.org/en-US/docs/Web/API/Element/setPointerCapture
- if the user pressed f11 on desktop browser disable the "exit fullscreen" button because it does 
  not work in this case
- sometimes when going fullscreen on mobile the canvas does not fully fill the part where the 
  statusbar would be. if we pull down the status bar the canvas grows to full size.
- Allow app to save files locally in wasm (browserdb?)
  - get rid of savegame folder on windows and just use appdata
- gamepad support for wasm
- Find out why gamepad shoulder trigger axes does not work. Directly accessing the state 
  with `Gamepad::axis_or_btn_name()` or iterating axis does not let us find any state. We know that 
  it should work because it does so in the MSWindows control panel
- make refresh rate snapping more smart (especially for deltatimes without vsync which is currently 
  wrong). (ie. we could use the values of the last ten frames as basis for snapping)

## better project structure and generator
- look for ways to simplify project creation and building
- assess which thirdpary tools we use for building/asset packing and document them and how to get them
  - aseprite
  - oggenc2
  - ResourceHacker
  - .. ?
  make assetbaker and buildtools crash with useful error message when those thirdparty tools do not
  exist and how to get them
- add more vscode tasks for wasm builds
- Get rid of crates that are not necessary or replace them with smaller/faster ones 
  - nanoserde, oorandom, minimp3, ...
  - get rid of sdl in favor of something more simple that does not require a separate dll to ship?
- look how other projects like bevy handle project templates
- rename game -> app
- Update version info resource with the crate version
- Zip up final executable and add version from crate
- We need to certify our Windows executable with a real certificate
  Maybe like this one:
  https://codesigncert.com/cheapcodesigning
  Also useful:
  https://social.technet.microsoft.com/wiki/contents/articles/38117.microsoft-trusted-root-certificate-program-participants-as-of-june-27-2017.aspx#C

# user experience
- find ways to make our wasm file smaller
- make app pause on onfocus/lost events more robust
  - show focus lost overlay "press here to continue"
  - give appcode a hint and some time to wind down and save state etc. on focus lost
  - let appcode respond with an ACK that it won't need to update anymore on focus lost
- add icon, title and tags to html (look at other projects we did)
- check out the output of https://realfavicongenerator.net/
- add splash screen on first run as html canvas background image + some "run game" icon
- check out how to make a manifest view-source:https://www.funkykarts.rocks/demo.html
- Make user facing panic messageboxes for wasm?
  - We need a production/develop version where we enable/disable i.e. panic messageboxes. It would be 
  useful to having a config file for this that is read on startup. Maybe this can be the same as the 
  display / controller config file? We want to configure/enable/disable the following things:
    - Show panics in messageboxes
    - Debug print frametimes
    - Set log levels
    - Splashscreen

## wasm performance
- Find more ways to make wasm perform better
- current hotspots are:
  - sorting drawables (they are pretty big to sort, maybe we can use references as payload?)
  - drawing rects by drawing bresenham lines 
- Find and get rid of needles allocations and copies
- Find out what causes garbage collector to trigger
- simplify and optimize audio rendering (less pipelining, bigger buffers, less copy, less iterators)

## Drawstate / Renderer
- evaluate what to do with DEFAULT_WORLD_ZNEAR and DEFAULT_WORLD_ZFAR constants that are duplicated
  in renderer and drawstate
- can we make the sorting faster? alternatively can we get rid of sorting for non-translucent 
  drawobjects by dividing them up into two drawbatches?
- add ability to add new shaders from drawstate

## Asset baker
- find out why our ogg decoder decodes more frames than exist in ogg file
- try another texture packer that is more efficient (maybe https://github.com/ChevyRay/crunch-rs
  or https://github.com/chinedufn/rectangle-pack)
  we must refactor our bitmapatlas packer pretty hard for this though as other packers assume 

## apps
* Repeaty:
  - When pressing start button and text input is empty (but previously valid) refill text input



---

# Archive

* Input, Fullscreen, Asset loading and GFX in WASM
* Pixie Stitch: 
  - Add custom launcher icon
* Add wasm audio
* Fix wasm performance
* bugfixing wasm audiobuffer
* pause game on focus lost
* keyboard input by just using a hashmap
* simplify touch api via hashmap. fix wasm touch api
* fullscreen toggle with dedicated screen button
* check if canvas/screen resolution is correct in fullscreen
- fix DOM error on fullscreen toggle by making screen orientation configurable (i.e. orientation change on desktop) (WONTFIX because of bad code complexity / usefulness ratio)
- find out why exiting fullscreen on mobile sometimes glitches and/or jumps back to fullscreen (using is_pressed instead of recently_pressed in code)
- find out why we need to double click our fullscreen button on wasm desktop (because mouseup events outside of browser window are not registered)
- find out if we can fix focus lost on leaving fullscreen on mobile 
  https://answers.unity.com/questions/282633/index.html suggests that we cant and should just implement
  focuslost/pause message screen very similar to initial startup screen

* fix wasm slowdown/crash when fast repeatetly touching canvas 
  (the problem was that we accessed the wrong finger hashmap)
* allow drawing in platform app launcher layer for debug purposes (on mobile wasm its difficult 
  to look at logs) WONTFIX because we can use edge browser to track logs made on mobile
* make drawstate call renderer functions directly? (NO THEN WE CAN'T EASILY REPLAY DRAWCOMMANDS 
  ON FOCUS LOST)
- make shader parser that knows all attributes and uniforms
- proper gl object encapsulation and lifetime management
- make one drawobjects per shader (maybe create from shader or tie more closely to shader?)
- shader compilation now returns results instead of panicking
- more errorchecking with debug names in renderer
- split drawcommand into buffer assignment and index drawing 
- Refactor draw/renderer to have one vertex-/index-batch-buffer per shader with offsets into buffer
  (see sokol_gfx appendbuffer mechanism)
- make vertexbuffers more save (disallow use of different vertex types) 
- pushing of drawables is now slightly faster ~10% -> ~4%
- splitting up some cottontail lib 
- updating dependencies
- replace rand with oorandom
- fixes audio bug in example scene
- adds audio debug visualization to example scene
- sdl layer now pauses on focus lost
- split audio out of ct lib
- replaces audrey crate with wav crate
- split up draw into draw and image crate
- WONTFIX Unify platform layers a bit to allow feature sharing (tried it and there very little in 
  common to justify the cost)
- move debugscene out of game lib and temporarily into launcher (later we want to make it a 
  standalone example)
- remove scenes concept and all its boilerplate
- removes target frametime concept, snaps deltatimes to nearest refresh rates
- let game use platform (reverse controlflow)
- split input out of ct lib into platform
- replaces wav crate with hound, adds wav encoding for debug purposes

- fix audio stuttering introduced earlier
- made window crate independant of audio crate
- make window crate independant of draw crate
- fixed vertexbuffer bug
- adds logs to renderer
- inverted controlflow draw->renderer
- make draw api more typesafe again 
  (NOTE: we did not put traits like vertex into the renderer because it would need things like
   Color to be known in the renderer)
- fix audio stuttering (performance?) on wasm (NOTE: buffer resize)
- fix copying of glyph-sprites when debug logging
- removes calls to html_get_canvas in mainloop
- caches all calls to html elements in wasm platform maybe we can use lazy static in core for each 
  imporant enough element (canvas, screen, window, document)?
  NOTE: We did not use lazy_static because it has unneeded atomic reference count overhead

- don't process streams if we know that they are empty
- replaces some .expect with .unrwap_or_else to avoid allocation
- WONTFIX: refactor audio system to use non interleaved buffers to comform more to 
  WebAudio API. It is not that practical to do it because we don't do that much processing
  and dynamic audio routing to justify the complexity
- make audio interpolator use pullbuffer method instead of being an iterator
- simplify our audiobuffers/sources/streams/mono/stero zoo
- fixes input recording
- greatly improved audio performance bt merging audiostream stages together
- fixes volume propagation of streams
- WONTFIX(build times would be too high and crate dependencies not clear) make crate controlflow 
  more streamlined (maybe build everything as one crate?)

- restored sprite debug scene
- added 3d sprite and spatial sound example

- adds ogg decoding for whole files

- convert wav files to ogg when assets baking
- adds audio metadata for assetbaker
- adds audio streaming of ogg files

- converting from interleaved processing to flat buffers per channel
- add ability to pack/load/play stereo audio files

- make resampler independent of audio source
- add final output resampler to resample internal hz rate to output hz rate (useful if we want 
  to render internally at 22050hz but output at 44100hz) and also use global 
  playback speed factor in final resampler
- automatically resample audio files to 44100Hz (desktop) and 22050Hz (wasm)
- distribute rendering of chunks over multiple frames at a constant rate instead of multiple chunks 
  in one frame to fill up the queue (important on wasm because we have bigger buffers there but even
  less time per frame)

- adds automatic download of tools in batchfiles (where possible)
- put all our generated project template files into a single directory and just copy it and replace 
  all containing strings in all templates. This could simplify our generator code immensely. maybe
  that way we can replace our system with (https://github.com/ffizer/ffizer)?
  NOTE: We rolled our own because there was not much needed to do for this simple change
- add new html/batchfiles and everything we added recently to the templates
- adds debug draw grid helper function
- adds debug crosshair drawing

- find out why our screenspace grid does not line up with our canvas space / worldspace objects
  ANSWER: The main problem is the canvas blit offset which is a non-zero percentage of a canvas 
          pixel when the camera's internal position is not pixel perfectly aligned. drawing things
          in screenspace and getting mouse coordinates from screenspace therefore currently has an 
          error
- remove the need to have a 'untextured.png' in assets folder
- now using graphics.data pack for graphics similar to audio.data for audio

- simplifies font asset baking by creating default metadata files if missing
- removes relative paths from asset names (this has the consequence that every resource of the same
  type needs to have a unique name)
- assetbaker now checks for font and audio metadata files without corresponding font/audio file
- adds content.data resource pack
- evaluate if we want to either get rid of the blit canvas offset (and don't have a smooth camera
  but get rid of much complexity) or implement the blit offset feature properly including
  an extra 1 pixel canvas padding and correct coordinate transformation from/to screenspace in the 
  camera struct which all need to use. Additionally we would need to overthink the canvas-space 
  drawing because it will either jitter when moving the camera or we need to draw the canvas space
  as a separate pass into its own framebuffer. but then we will have pixels between canvas and world
  that won't align properly. 
  - NOTE: Our dungeontracks and many other games like Celeste and Downwell don't use pixel smoothing
          and it looks ok!
- gets rid of screenspace blit offset because it adds too much complexity
- bake a minimal graphics pack with splashscreen only
- move controllerdb into executable on sdl2 platform

- use prelude graphics pack that loads quickly to show splashscreen
- improve wasm startup speed (load graphic assets first to show splash screen, then later sound assets)
- adds loadingscreen progressbar
- reuse drawtext method for draw debug logging
- Make texture packer size dynamically growing up to a maximum size of 4096
- rudimentary hotloading of assets on desktop
- Clean up old stuff code at the end of draw.rs and sdl_window.rs. Determine what is needed and 
  implement it (drawing the depthbuffer and various debug grids should be useful). Throw out the rest 
- Fix screenspace coordinate transformation for cases where we have letterboxing

- Fixes warnings
- use drawparams instead of depth,color,additivity,drawspace tuple
- adds some helper functions to help with text alignment