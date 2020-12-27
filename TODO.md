# DONE:

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

# CURRENT


# NEXT:

- refactor audio system to use non interleaved buffers to comform more to WebAudio API
- don't process streams if we know that they are empty

## better platform layer
- rename things that are not necessarily game related to app
- also do we need resize callbacks at all? (also in sdl2)?
- fix mouseup/touchup events that happen outside of browser window (i.e. affects leaving fullscreen)
  we may need https://developer.mozilla.org/en-US/docs/Web/API/Element/setPointerCapture
- if the user pressed f11 on desktop browser disable the "exit fullscreen" button because it does 
  not work in this case
- sometimes when going fullscreen on mobile the canvas does not fully fill the part where the 
  statusbar would be. if we pull down the status bar the canvas grows to full size.
- Allow app to save files locally (browserdb?)
  - get rid of savegame folder on windows and just use appdata
- gamepad support for wasm
- Find out why gamepad shoulder trigger axes does not work. Directly accessing the state 
  with `Gamepad::axis_or_btn_name()` or iterating axis does not let us find any state. We know that 
  it should work because it does so in the control panel
- make refresh rate snapping more smart (especially for deltatimes without vsync which is currently 
  wrong). (ie. we could use the values of the last ten frames as basis for snapping)

## writing games easier
- make draw/audio/other things global for easier use (we run everything on the same thread anyway)
- Easier text drawing api
  - one that is simple without much parameters
  - one that just centers text in rect
  - other ideas?
- Ability to draw debug graphs to i.e. try out attenuation for audio distance
- Easy debug-printing text API that draws in screenspace (not canvas-space)
- We need to add a debug layer to the drawstate with its own drawqueue
- make crate controlflow more streamlined (maybe build everything as one crate?)
- get rid of scenes system and game events
- add unified/virtual gamecursor input to gamecursors struct (uses mouse or first touchfinger)
- simplify query for finger press events
- Change linestrip drawing api to take a `loop` parameter so we can get rid of 5 vertex 
  sized rectangle drawing and the `skip_last_vertex` 
- Fix Vec2 to work with flipped_y only and remove special suffixes?
- Add modulators like in https://www.youtube.com/watch?v=n-txrCMvdms especially shift register 
  modulator and newtonian following modulator
- Future tutorial games:
  - https://simplegametutorials.github.io/
  - https://github.com/noooway/love2d_arkanoid_tutorial
  - https://github.com/adnzzzzZ/blog/issues/30
  - https://inventwithpython.com/blog/2012/02/20/i-need-practice-programming-49-ideas-for-game-clones-to-code/
  - https://gamedev.stackexchange.com/a/945
  - https://www.gamedev.net/articles/programming/general-and-gameplay-programming/your-first-step-to-game-development-starts-here-r2976
  - https://bfnightly.bracketproductions.com/rustbook/chapter_0.html

## better project structure and generator
- Get rid of crates that are not necessary or replace them with smaller/faster ones 
- nanoserde, oorandom, minimp3, ...
- get rid of sdl in favor of something more simple?
- look how other projects like bevy handle project templates
- convert debug scene to example in a dedicated examples folder with its own assets and build scripts
- rename game -> app
- look for ways to simplify project creation and building
- add new html/batchfiles and everything we added recently to the templates
- add more vscode tasks for wasm builds
- Update version info resource with the crate version
- Zip up final executable and add version from crate
- We need to certify our Windows executable with a real certificate
  Maybe like this one:
  https://codesigncert.com/cheapcodesigning
  Also useful:
  https://social.technet.microsoft.com/wiki/contents/articles/38117.microsoft-trusted-root-certificate-program-participants-as-of-june-27-2017.aspx#C

## renderer flexibility + speed + cleanup
- add ability to add new shaders from drawstate
- Clean up old stuff code at the end of draw.rs and sdl_window.rs. Determine what is needed and implement it. Throw out the rest 

## improve asset loading
- refactor gamememory/audio/draw/asset initialization to 
  - allow hotloading of assets
  - improve wasm startup speed

## audio system
- find a way to make pulling buffers more convenient, performant and generic (i.e. linear 
  interpolators use `next()` on their interal source to get the next sample)
- use global playback speed factor in a final output interplator not
- Support ogg audio and differentiate between mono/stereo recordings
- streaming long audio (music)

# user interface
- make app pause on onfocus/lost events more robust
- show focus lost overlay "press here to continue"
- give appcode a hint and some time to wind down and save state etc.
- let appcode respond with an ACK that it won't need to update anymore
- only replay drawcommands that are don't allocate resources
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
- test out 22050khz audio?
- current hotspots are:
  - sorting drawables (they are pretty big to sort, maybe we can use references as payload?)
  - drawing rects by drawing bresenham lines 
  - render audiochunks 
- Find and get rid of needles allocations and copies
- Find out what causes garbage collector to trigger
- simplify and optimize audio rendering (less pipelining, bigger buffers, less copy, less iterators)

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