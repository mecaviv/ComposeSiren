
# VST3 should be built on all platforms
set (FormatsToBuild VST3 Standalone)
set(CMAKE_OSX_ARCHITECTURES "x86_64;arm64" CACHE INTERNAL "")

# On Mac, a AU version will be built too
if (APPLE)
    list (APPEND FormatsToBuild AU)
endif()

# If a path to the AAX SDK is passed to CMake, an AAX version will be built too
if (AAX_SDK_PATH)
    juce_set_aax_sdk_path (${AAX_SDK_PATH})
    list (APPEND FormatsToBuild AAX)
endif()

juce_add_binary_data(assets SOURCES
        #[[
        ../Resources/dataAmpS1
        ../Resources/dataAmpS3
        ../Resources/dataAmpS4
        ../Resources/dataAmpS5
        ../Resources/dataAmpS7
        ../Resources/dataFreqS1
        ../Resources/dataFreqS3
        ../Resources/dataFreqS4
        ../Resources/dataFreqS5
        ../Resources/dataFreqS7
        ../Resources/datadureTabsS1
        ../Resources/datadureTabsS3
        ../Resources/datadureTabsS4
        ../Resources/datadureTabsS5
        ../Resources/datadureTabsS7
        ../Resources/dataVectorIntervalS1
        ../Resources/dataVectorIntervalS2
        ../Resources/dataVectorIntervalS3
        ../Resources/dataVectorIntervalS4
        ../Resources/dataVectorIntervalS5
        ../Resources/dataVectorIntervalS7
        ]]

        ../Resources/Picto_Siren_40x37.png
        )

function(package_resources target folder)
    set(absfolder ${folder})
    _juce_make_absolute(absfolder)
    if(NOT EXISTS "${absfolder}")
        message(FATAL_ERROR "Could not find resource folder ${absfolder}")
    endif()

    message("folder: " ${folder} " absfolder: " ${absfolder} "   bindir " ${CMAKE_CURRENT_BINARY_DIR})
    get_filename_component(folder_parent_path "${folder}" DIRECTORY)
    file(GLOB_RECURSE resources RELATIVE "${absfolder}" "${absfolder}/data*")
    #file(GLOB_RECURSE resources "${folder}/data*")
    set_target_properties(${target} PROPERTIES BUNDLE True)
    get_target_property(targetbundle ${target} BUNDLE)

    message("bundle: ${targetbundle}")
    #file(GLOB_RECURSE resources "${absfolder}/data*")
    foreach(file IN LISTS resources)
        set(resourcefile ${folder}/${file})
        message("resource file " ${resourcefile} "   ->" ${file} "----> " "${folder}/${file}")
        target_sources(${target} PRIVATE ${resourcefile})
        set_source_files_properties(${resourcefile} PROPERTIES
                HEADER_FILE_ONLY TRUE
                MACOSX_PACKAGE_LOCATION "Resources")
        #set_property(SOURCE ${resourcefile} PROPERTY MACOSX_PACKAGE_LOCATION "Resources")
        #[[

        set_target_properties(${target} PROPERTIES
                MACOSX_BUNDLE TRUE
                RESOURCE ${resourcefile})
        set_source_files_properties(${resourcefile} PROPERTIES
                HEADER_FILE_ONLY TRUE
                MACOSX_PACKAGE_LOCATION "Resources/${file}")

        get_source_file_property(sourcefileprops "${resourcefile}" MACOSX_PACKAGE_LOCATION)
        message("file props: " ${sourcefileprops} )
        SET_SOURCE_FILES_PROPERTIES(${resourcefile} PROPERTIES MACOSX_PACKAGE_LOCATION Resources)

        set_property(SOURCE ${resourcefile} PROPERTY MACOSX_PACKAGE_LOCATION "Resources/${file}")
]]
    endforeach()
    SET_TARGET_PROPERTIES(${target} PROPERTIES MACOSX_BUNDLE TRUE)
endfunction()

#[[
function(juce_add_bundle_resources_directory target folder)
    _juce_make_absolute(folder)

    if(NOT EXISTS "${folder}")
        message(FATAL_ERROR "Could not find resource folder ${folder}")
    endif()

    get_filename_component(folder_parent_path "${folder}" DIRECTORY)
    file(GLOB_RECURSE resources RELATIVE "${folder_parent_path}" "${folder}/*")
    message("folder parent path: " ${folder_parent_path})
    foreach(file IN LISTS resources)
        message("file resource:" ${file})
        target_sources(${target} PRIVATE "${folder_parent_path}/${file}")
        get_filename_component(resource_parent_path "${file}" DIRECTORY)
        message("resource parent path: " ${resource_parent_path} "eee")

        # gauthier: edge case when file is at root of a folder and folder is named Resources.
        if("Resources" STREQUAL ${resource_parent_path})
            set(resource_parent_path "")
        endif()
        set_source_files_properties("${folder_parent_path}/${file}" PROPERTIES
                HEADER_FILE_ONLY TRUE
                MACOSX_PACKAGE_LOCATION "Resources/${resource_parent_path}")
    endforeach()
endfunction()
#]]
juce_add_plugin(ComposeSiren

        COMPANY_NAME "MecaniqueVivante"
        PRODUCT_NAME "ComposeSiren"
        FORMATS ${FormatsToBuild}
        AAX_IDENTIFIER com.mecaviv.ComposeSiren
        # A four-character manufacturer id with at least one upper-case character
        PLUGIN_MANUFACTURER_CODE McVv
        # A unique four-character plugin id with at least one upper-case character
        PLUGIN_CODE MvCs
        IS_SYNTH TRUE                       # Is this a synth or an effect?
        NEEDS_MIDI_INPUT TRUE               # Does the plugin need midi input?
        NEEDS_MIDI_OUTPUT TRUE              # Does the plugin need midi output?
        IS_MIDI_EFFECT FALSE                 # Is this plugin a MIDI effect?
        EDITOR_WANTS_KEYBOARD_FOCUS TRUE    # Does the editor need keyboard focus?
        COPY_PLUGIN_AFTER_BUILD TRUE        # Should the plugin be installed to a default location after building?
        #VST3_CATEGORIES Fx Distortion
        #AU_MAIN_TYPE kAudioUnitType_Effect
        #AAX_CATEGORY AAX_ePlugInCategory_Harmonic
        )

#juce_add_bundle_resources_directory(ComposeSiren ../Resources)
package_resources(ComposeSiren "../Resources")

target_compile_definitions(ComposeSiren PRIVATE CSBUILD_WITH_CMAKE=1)

target_compile_definitions(ComposeSiren
        PUBLIC
        # JUCE_WEB_BROWSER and JUCE_USE_CURL would be on by default, but you might not need them.
        JUCE_WEB_BROWSER=0  # If you remove this, add `NEEDS_WEB_BROWSER TRUE` to the `juce_add_plugin` call
        JUCE_USE_CURL=0     # If you remove this, add `NEEDS_CURL TRUE` to the `juce_add_plugin` call
        JUCE_VST3_CAN_REPLACE_VST2=1
        )
message(STATUS "found samplerate:? " "${SampleRate_FOUND}")
if(SampleRate_FOUND)
    target_compile_definitions(ComposeSiren PRIVATE CSBUILD_HAS_RESAMPLING_WITH_LIBSAMPLERATE=1)
    get_target_property(_if_compile_defs ComposeSiren
            INTERFACE_COMPILE_DEFINITIONS)

    get_target_property(_compile_defs ComposeSiren COMPILE_DEFINITIONS)
    message(STATUS "setting resampling flag------------------------" ${_if_compile_defs})
    message(STATUS ${_if_compile_defs})
    message(STATUS "-------------------------------")
    message(STATUS ${_compile_defs})
    target_include_directories(ComposeSiren PRIVATE ${SampleRate_INCLUDE_DIR})
endif()
message(STATUS "sr incl dir:" "${SampleRate_INCLUDE_DIR}")
if(Boost_FOUND)
    message(STATUS "boost found")
endif()
message(STATUS "boost incl dir:" "${Boost_headers_INCLUDE_DIR}")

target_sources(ComposeSiren PRIVATE
        CS_midiIN.cpp
        PluginEditor.cpp
        PluginProcessor.cpp
        Sirene.cpp
        synth.cpp
        gui-juce/SireneSliders.cpp
        plugin-juce/SireneAudioProcessorParameters.cpp
        )

target_link_libraries (ComposeSiren PRIVATE
        # to include binary assets
        assets
        # JUCE Modules
        #juce::core
        juce::juce_analytics
        juce::juce_audio_basics
        juce::juce_audio_devices
        juce::juce_audio_formats
        juce::juce_audio_plugin_client
        juce::juce_audio_processors
        juce::juce_audio_utils
        juce::juce_box2d
        juce::juce_core
        juce::juce_cryptography
        juce::juce_data_structures
        juce::juce_dsp
        juce::juce_events
        juce::juce_graphics
        juce::juce_gui_basics
        juce::juce_gui_extra
        juce::juce_opengl
        juce::juce_osc
        juce::juce_product_unlocking
        juce::juce_video
        # Recommended flags
        juce::juce_recommended_lto_flags
        juce::juce_recommended_warning_flags
        juce::juce_recommended_config_flags

        # Binary Data
        #        EmbeddedBinaryData

        # for resampling to DAW sample frequency
        #samplerate
        SampleRate::samplerate
        )
juce_generate_juce_header(ComposeSiren)
