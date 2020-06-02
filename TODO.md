# DONE:

* Encapsulate audio rendering, callback and atomics into AudioOutput struct. the render function
  of AudioOutput should take a &Audiostate
* Restore Audiostate to its previous functionalty
* implement (linear) resampler stream adapter and/or mixixing with resampling
* DungeonTracks
  - Add movement sound
* Make audio fader in platformlayer commit fully to fading in or out completely so i.e. we don't 
  fade out halfway after loosing frames and fade from 0.5 back to 1.0 instead from 0.0 to 1.0

# CURRENT

* Simplify Platform Audio
  - Get rid of dynamically allocated buffers

* DungeonTracks
  -

# NEXT:

* Add spatial audio
* Add new method to GameMemory to simplify update method
* Maybe we can make drawstate globally available for debug drawing so that we don't need it to 
  pass everywhere. this is of course highly unsafe but ok for debug
  


* Repeaty:
  - When pressing start button and text input is empty (but previously valid) refill text input
    
* Pixie Stitch: 
  - Add custom launcher icon


* Change linestrip drawing api to take a `loop` parameter so we can get rid of 5 vertex 
  sized rectangle drawing and the `skip_last_vertex` 
* Fix Vec2 to work with flipped_y only and remove special suffixes
* Easy debug-printing text API that draws in screenspace (not canvas-space)
  - We need to add a debug layer to the drawstate with its own drawqueue

* Find out why gamepad shoulder trigger axes does not work. directly can access the state 
  with `Gamepad::axis_or_btn_name()` or iterating axis does not let us find any state. We know that 
  it should work because it does so in the control panel


* We need a production/develop version where we enable/disable i.e. panic messageboxes. It would be 
  useful to having a config file for this that is read on startup. Maybe this can be the same as the 
  display / controller config file? We want to configure/enable/disable the following things:
  - Show panics in messageboxes
  - Debug print frametimes
  - Set log levels
  - Splashscreen
* Clean up old stuff code at the end of draw.rs and sdl_window.rs. 
  Determine what is needed and implement it. Throw out the rest 

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


