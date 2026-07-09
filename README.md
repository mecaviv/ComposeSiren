# ComposeSiren

**ComposeSiren** is a suite of 2 audio and MIDI virtual instrument plugins that synthesize sounds of musical sirens
made by [Mécanique Vivante][1].
Each plugin provides a set of parameters automatable from DAW hosts and synchronized to its MIDI input and output.
Together, they allow to compose pieces for Mécanique Vivante's siren orchestra in studio by replicating their
behaviour : the CC and Pitchwheel messages mirror the MIDI control parameters of the real sirens, and their physical
properties and sound are simulated in real-time from actual captured data.
The DAW projects can ultimately be reused to play the pieces on the orchestra during live performances by controlling
the sirens from the DAW's MIDI output.

The orchestra is composed of 7 MIDI sirens :
- two altos (S1 and S2),
- a bass (S3),
- a tenor (S4),
- two sopranos (S5 and S6),
- a piccolo (S7).

### OneSiren

A simple plugin with flexible MIDI routing parameters that can simulate any siren from the orchestra.
![OneSiren plugin](./Doc/pics/Mecaviv-OneSiren-soprano-01.png)

### SirenOrchestra

A plugin that simulates the whole orchestra with its original fixed MIDI routing, and provides additional controls
such as panning and volume adjustment for each simulated siren, as well as an embedded reverberation module and a master
volume control.
![SirenOrchestra](./Doc/pics/Mecaviv-SirenOrchestra-tenor-01.png)

On MacOS, the plugins are available as universal (x86_64/arm64) 64 bit VST3, Audio Unit and Standalone Application
formats. On windows they are available as x64 VST3 and Standalone Application formats.
They are currently tested on [Reaper][6] and [Ableton Live][4].

The ComposeSiren suite is developed on top of the **JUCE** frameworks. You can find more infos about it there: http://www.juce.com.

## Getting the plugins

Download the latest installer for your OS from the [releases page](https://github.com/mecaviv/ComposeSiren/releases)
and run it. This will install both plugins in the formats available on your platform, and a bunch of shared resource
files required for the simulation.

## Build instructions

The project is based on `CMake`.

In orger to build it, you should have **`CMake 3.22+`**,
a **`C++20`** compiler, and (for optional Resource files processing build step)
**`Python 3`** installed on your system.  
It is also recommended to have `Ninja` but you can use `XCode` and
`Visual Studio` generators for MacOS and Windows respectively.
You will also need `NSIS` for Windows installer generation.

The project consumes a few variables listed in the template config file `Config.cmake`
(VST2 SDK path and various credentials for software signing)

You can derive your own `MyConfig.cmake` file from it, then run the following
commands:
```
$ cmake -B <my_cmake_build_dir> -DCMAKE_BUILD_TYPE=<my_build_type> -C MyConfig.cmake
$ cmake --build <my_cmake_build_dir> --target <my_target>
```

Example packaging commands for MacOS with XCode:
```
$ cmake -B cmake-build-release -DCMAKE_BUILD_TYPE=RELEASE -C MyConfig.cmake
$ cmake --build cmake-build-RELEASE --target dist
```

Example packaging commands for Windows with Visual Studio 2022:
```
$ cmake -B cmake-build-release -G "Visual Studio 17 2022" -C MyConfig.cmake
$ cmake --build cmake-build-release --config Release --target dist
```

The resulting installer (built with `productbuild` on mac and `NSIS` on windows)
is created in `build/Packaging/ComposeSiren_Installer_artefacts`

### dependencies

#### linux

```sh
sudo apt-get install libx11-dev libxrandr-dev libxinerama-dev libxcursor-dev libfreetype-dev
```

#### Raspberry Pi

```sh
sudo apt install cmake libxrandr-dev libxinerama-dev libxcursor-dev libfreetype6-dev libasound2-dev
```

Resource path is hardcoded to point to `/home/sirenateur/Documents/src/mecaviv/ComposeSiren/Resources/`, so please checkout the repository in `/home/sirenateur/Documents/src/mecaviv/`:

```shell
cd ~/Documents
mkdir src
cd src
mkdir mecaviv
cd mecaviv
git clone https://github.com/mecaviv/ComposeSiren.git
```

### git tips

* first clone the repository with the `--recursive` option to fetch JUCE
  submodule, or run `git submodule update --init` after cloning.
* if at some point the `Dependencies/JUCE` submodule is altered by some IDE, you
  can reset it using `git submodule deinit -f .` then `git submodule update --init`

### NB :
* download VS 2022 Community from [HERE](https://aka.ms/vs/17/release/vs_community.exe)
* more dl links (MSBuildTools, VS versions) [HERE](https://sharethis.zip/visual_studio/)

[1]: https://mecanique-vivante.com/en/instrumental-exploration/
[2]: https://minhaskamal.github.io/DownGit/#/home?url=https://github.com/patriceguyot/ComposeSiren/tree/master/Builds/MacOSX/ComposeSiren.vst3
[3]: https://help.ableton.com/hc/en-us/sections/202295165-Plug-Ins
[4]: https://www.ableton.com/en/live/
[5]: https://minhaskamal.github.io/DownGit/#/home?url=https://github.com/patriceguyot/ComposeSiren/tree/master/Builds/MacOSX/ComposeSiren.component
[6]: https://www.reaper.fm/
