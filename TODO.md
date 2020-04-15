## Done today

* Assets baker always generates default fonts
* Can use build-in and loaded fonts the same way
* Removed the need to pass font filepath to drawstate on its creation
* Added json font style file which contains info on how to render the given font
* Added usage of a loaded font to main example
* Fixed font metrics calculations via offsets
* Draw baseline in font test in main
* Tested all proggy fonts and set their correct offsets and sizes


## Current:


## Next:

* Move font rendering tests in main to debugscene and pass a loaded fontname 
* When calculating the dimensions of text we currently get back the lineheight even if we only 
  have glyphs like `'a'` in out text. When trying to center this text vertically we get the wrong results
* How can we share the draw and measure algorithms of our `Font` and `BitmapFont` structs?

## Backlog:

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


