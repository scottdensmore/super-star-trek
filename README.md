# Super Star Trek

Super Star Trek is an old text-only game, an early example of a turn-based space strategy sim, written in BASIC. In this game, you are the captain of the starship Enterprise, and your mission is to scout the federation space and eliminate all the invading Klingon ships. You will have to manage the ship energy carefully, use phasers and torpedoes to destroy the Klingons, and find starbases to repair damages and replenish your energy. All of this, rendered with a few characters on screen and a lot of imagination.

Super Star Trek is probably the most famous early text-only game ever created and it inspired many other videogames. Since the code was public domain, during the years, it was changed and improved many times. There are literally thousands of versions out there. All of them are nice, but my preference goes to the classic 1978 version called Super Star Trek.

[Read more about Star Trek and Super Star Trek.](https://en.wikipedia.org/wiki/Star_Trek_%281971_video_game%29)

### Building for Windows
For compiling with Visual Studio, you don't need to run Visual Studio but use the Developer Command Prompt.
From the start menu run the Developer Command Prompt for Visual Studio  and cd to where the sst source files are.
Then run the command "cl /DWINDOWS /Fesst.exe *.c"
This works if the only C source files are those of SST, otherwise you will need to list all the source file names
in the command line

