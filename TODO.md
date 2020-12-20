# DONE:

* Input, Fullscreen, Asset loading and GFX in WASM
* Pixie Stitch: 
  - Add custom launcher icon
* Add wasm audio


# CURRENT

* Fix wasm performance
  - Get rid of needles allocations
  - Find out what causes garbage collector to trigger
  - simplify and optimize audio rendering (less pipelining, bigger buffers, less copy, less iterators)

# NEXT:

* make app pause on onfocus/lost events 
* Get rid of crates that are not necessary or replace them with smaller ones
  (nanoserde, nanorand, minimp3)
* check canvas resolution in fullscreen
* fix DOM error on fullscreen toggle
* refactor gamememory/audio/draw/asset initialization
* make crate controlflow more streamlined (maybe build everything as one crate?)
* rename game -> app
* get rid of savegame folder
* add new batchfiles and everything we added recently to the templates
* simplify project creation
* add icon, title and tags to html (look at other projects we did)
* get rid of sdl in favor of something more simple?
* gamepad support for wasm
* simplify keyboard input
* we need a sane way to determine refresh rate and calculate target_update_rate
* make draw/audio/other things global for easier use (we run everything on the same thread anyway)
* Maybe we can make drawstate globally available for debug drawing so that we don't need it to 
  pass everywhere. this is of course highly unsafe but ok for debug
* Easy debug-printing text API that draws in screenspace (not canvas-space)
  - We need to add a debug layer to the drawstate with its own drawqueue
* Refactor draw/renderer to have one vertex-/index-batch-buffer per shader with offsets into buffer
  (see sokol_gfx appendbuffer mechanism)
* Change linestrip drawing api to take a `loop` parameter so we can get rid of 5 vertex 
  sized rectangle drawing and the `skip_last_vertex` 
* Fix Vec2 to work with flipped_y only and remove special suffixes?
* Ability to draw debug graphs to i.e. try out attenuation for audio distance
* Find out why gamepad shoulder trigger axes does not work. Directly accessing the state 
  with `Gamepad::axis_or_btn_name()` or iterating axis does not let us find any state. We know that 
  it should work because it does so in the control panel
* Usable wasm/html buttons for fullscreen switching
* Allow hotreloading of game assets
* Support ogg audio and differentiate between mono/stereo recordings
* streaming long audio (music)
* Clean up old stuff code at the end of draw.rs and sdl_window.rs. 
  Determine what is needed and implement it. Throw out the rest 
* We need a production/develop version where we enable/disable i.e. panic messageboxes. It would be 
  useful to having a config file for this that is read on startup. Maybe this can be the same as the 
  display / controller config file? We want to configure/enable/disable the following things:
  - Show panics in messageboxes
  - Debug print frametimes
  - Set log levels
  - Splashscreen


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



* Repeaty:
  - When pressing start button and text input is empty (but previously valid) refill text input