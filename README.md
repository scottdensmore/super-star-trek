# Super Star Trek

### Building for macOS and Linux



### Building for Windows
For compiling with Visual Studio, you don't need to run Visual Studio but use the Developer Command Prompt.
From the start menu run the Developer Command Prompt for Visual Studio  and cd to where the sst source files are.
Then run the command "cl /DWINDOWS /Fesst.exe *.c"
This works if the only C source files are those of SST, otherwise you will need to list all the source file names
in the command line

