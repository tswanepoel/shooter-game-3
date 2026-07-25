mouse move updates camera fine. but super slow left/right mouse movements appeared to have no effect: the camera remained frozen.

the problem vanished when disabling the call to set_look from within render loop (see crates\game-client\src\client\frame\mod.rs, line 81)
