cmake_minimum_required(VERSION 3.20)
project(ComposeSiren VERSION 0.0.1)
set_property(GLOBAL PROPERTY CXX_STANDARD 20)
# Don't add this line if you will try_compile with boost.
set(CMAKE_CXX_STANDARD_REQUIRED ON)

#set(CMAKE_MODULE_PATH cmake/Modules)
#include(cmake/Modules/libsamplerate-config.cmake)

find_package(JUCE CONFIG REQUIRED)
find_package(SampleRate CONFIG REQUIRED)

if(WIN32)
    # for Windows operating system in general
    juce_set_vst2_sdk_path("C:\\dev\\src\\steinberg\\VST3_SDK\\")
endif()
if(APPLE)
    juce_set_vst2_sdk_path(/Users/gauthiersegay/dev/src/steinberg/VST3_SDK)
endif()

add_subdirectory (Source)

#Adds all the module sources so they appear correctly in the IDE
set_property(GLOBAL PROPERTY USE_FOLDERS YES)
option(JUCE_ENABLE_MODULE_SOURCE_GROUPS "Enable Module Source Groups" ON)
