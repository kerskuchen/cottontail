## Done today

* Port aligned drawing functionality from `BitmapFont` to `Font`
* Make fonts consistently use either Vec2i or Vec 
* Move bitmap font drawing to `Bitmap`
* Refactored other Font functions to use the iterators
* Prevent bitmapfonts to be created with glyphs that have negative horizontal or vertical offset
* Simplify font tester function
* Solved problem with rogue glyphs that are not drawn between [0, font_height - 1]
* Move `pixelsnapped` functions into `Vec2` and `Rect`
* Make pixie_stitch use new font api

## Next:

* Replace git subtrees because it is getting dangerous
* test

## Backlog:

* Prefix all generic files like `main.rs`, `lib.rs`, `mod.rs` with its module/cratename
* Create static assets dir concept that is not processed and just copied directly to assets_baked
* Create build_shipping.bat that copies assets_baked and launcher.exe into ouput dir
* Rename main executable to launcher

* Should we refactor clipping glyph iterator to use basic iterator internally?

* Read the following items in the launcher from a config file:
  - display_to_use;
  - deadzone_threshold_x;
  - deadzone_threshold_y;
* We need a production/develop version where we enable/disable i.e. panic messageboxes. It would be 
  useful to having a config file for this that is read on startup. Maybe this can be the same as the 
  display / controller config file? We want to configure/enable/disable the following things:
  - Show panics in messageboxes
  - Debug print frametimes
  - Set log levels
  - Splashscreen
* How can we make GAME_WINDOW_TITLE, GAME_SAVE_FOLDER_NAME and GAME_COMPANY_NAME available to the
  platform layer in a much more convenient way? Maybe we can use include_bytes! macro?
* Clean up old stuff code at the end of draw.rs and sdl_window.rs. 
  Determine what is needed and implement it. Throw out the rest 
* Easy debug-printing text API that draws in screenspace (not canvas-space)
  - We need to add a debug layer to the drawstate with its own drawqueue
* Allow hotreloading of game assets
* Support ogg audio and differentiate between mono/stereo recordings
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


