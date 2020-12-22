# DONE:

* Input, Fullscreen, Asset loading and GFX in WASM
* Pixie Stitch: 
  - Add custom launcher icon
* Add wasm audio
* Fix wasm performance
* bugfixing wasm audiobuffer
* pause game on focus lost
* keyboard input by just using a hashmap
* simplify touch api via hashmap. fix wasm touch api


# CURRENT

* fullscreen toggle with dedicated screen button
  - fix DOM error on fullscreen toggle by making screen orientation configurable (i.e. orientation change on desktop)
  - check if canvas/screen resolution is correct in fullscreen
  - do we need resize callbacks at all? (also in sdl2)?
  - find out why we need to double click our fullscreen button on wasm
  - find out why exiting fullscreen on mobile sometimes glitches and/or jumps back to fullscreen
  - do we need event.prevent_default()? if yes where? putting it on touch events just ruins the 
    focus handling

# NEXT:


* make app pause on onfocus/lost events more robust
  - give appcode a hint and some time to wind down and save state etc.
  - let appcode respond with an ACK that it won't need to update anymore
  - only replay drawcommands that are don't allocate resources

* add icon, title and tags to html (look at other projects we did)
  - check out the output of https://realfavicongenerator.net/
  - add splash screen on first run as html canvas background image + some "run game" icon
  - check out how to make a manifest view-source:https://www.funkykarts.rocks/demo.html

* Refactor draw/renderer 
  - to have one vertex-/index-batch-buffer per shader with offsets into buffer
    (see sokol_gfx appendbuffer mechanism)
  - move shaders out of renderer and into draw, 
  - make shader parser that knows all attributes and uniforms
  - Clean up old stuff code at the end of draw.rs and sdl_window.rs. Determine what is needed and implement it. Throw out the rest 

* Find more ways to make wasm perform better
  - Get rid of needles allocations and copies
  - Find out what causes garbage collector to trigger
  - simplify and optimize audio rendering (less pipelining, bigger buffers, less copy, less iterators)

* Get rid of crates that are not necessary or replace them with smaller/faster ones 
  - nanoserde, nanorand, minimp3, ...
  - get rid of sdl in favor of something more simple?

* refactor gamememory/audio/draw/asset initialization
  - Allow hotreloading of game assets

* make crate controlflow more streamlined (maybe build everything as one crate?)
  - rename game -> app
  - make draw/audio/other things global for easier use (we run everything on the same thread anyway)
  - make drawstate call renderer functions directly? (NO THEN WE CAN'T EASILY REPLAY DRAWCOMMANDS ON FOCUS LOST)
  - get rid of scenes system and game events
  - move debug scene to examples folder with its own assets and build scripts

* Allow app to save files locally
  - get rid of savegame folder on windows and just use appdata

* look for ways to simplify project creation and building
  - look how other projects like bevy handle project templates
  - add new html/batchfiles and everything we added recently to the templates
  - add more vscode tasks for wasm builds
  - Update version info resource with the crate version
  - Zip up final executable and add version from crate

* we need a sane way to determine refresh rate and calculate target_update_rate

* add unified/virtual gamecursor input to gamecursors struct (uses mouse or first touchfinger)
  - simplify touch query for press events

* Easier text drawing api
  - one simple without much parameters
  - one center in rect
* Ability to draw debug graphs to i.e. try out attenuation for audio distance
  - Easy debug-printing text API that draws in screenspace (not canvas-space)
  - We need to add a debug layer to the drawstate with its own drawqueue

* Change linestrip drawing api to take a `loop` parameter so we can get rid of 5 vertex 
  sized rectangle drawing and the `skip_last_vertex` 

* Fix Vec2 to work with flipped_y only and remove special suffixes?

* Support ogg audio and differentiate between mono/stereo recordings
  - streaming long audio (music)

* gamepad support for wasm
  - Find out why gamepad shoulder trigger axes does not work. Directly accessing the state 
    with `Gamepad::axis_or_btn_name()` or iterating axis does not let us find any state. We know that 
    it should work because it does so in the control panel

* Make user facing panic messageboxes for wasm?
  - We need a production/develop version where we enable/disable i.e. panic messageboxes. It would be 
  useful to having a config file for this that is read on startup. Maybe this can be the same as the 
  display / controller config file? We want to configure/enable/disable the following things:
    - Show panics in messageboxes
    - Debug print frametimes
    - Set log levels
    - Splashscreen

* Add modulators like in https://www.youtube.com/watch?v=n-txrCMvdms especially shift register 
  modulator and newtonian following modulator

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