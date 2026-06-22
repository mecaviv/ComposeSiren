# this is a blank config file where every build environment specific information
# can be specified.
# create your own (e.g. MyConfig.cmake) and gitignore it.

# PATH TO VST 2 SDK ############################################################

# absolute path to the VST2 SDK, something like /<some_dirs>/vstsdk2.4
# VST2 target will not be built if not specified
set(VST2_PATH
  ""
  CACHE STRING "Path to VST2 SDK"
)

# Do the plugins have common resources to install ? ############################

#set(PLUGIN_RESOURCES_DIR
#  "Resources"
#  CACHE STRING "Top level directory that should be installed as shared plugin resources"
#)

# Source of truth
set(RAW_RESOURCES_DIR
        "${CMAKE_SOURCE_DIR}/Resources"
        CACHE PATH "Original resources directory"
)

# Folder where
set(PROCESSED_RESOURCES_DIR
        "${CMAKE_BINARY_DIR}/Resources-processed"
        CACHE PATH "Processed resources output directory"
)

# Do we generate and use new resource files with python ?
option(USE_PROCESSED_RESOURCES
        "Use ResourcesProcessing outputs instead of raw Resources"
        ON
)

# Unified variable used across packaging scripts
if(USE_PROCESSED_RESOURCES)
    set(PLUGIN_RESOURCES_DIR "${PROCESSED_RESOURCES_DIR}" CACHE PATH "Resources directory for packaging" FORCE)
else()
    set(PLUGIN_RESOURCES_DIR "${RAW_RESOURCES_DIR}" CACHE PATH "Resources directory for packaging" FORCE)
endif()


# SIGNING CREDENTIALS ##########################################################

# we could sign from xcode project but we want to allow using other generators :
# set(CMAKE_XCODE_ATTRIBUTE_CODE_SIGN_IDENTITY "firstname lastname") # (or apple id ?)
# set(CMAKE_XCODE_ATTRIBUTE_DEVELOPMENT_TEAM "ABCDE12345")

# the string id of your apple developer id application certificate
# something like "Developer ID Application: firstname lastname (ABCDE12345)"
set(APPLE_DEVELOPER_ID_APPLICATION
  ""
  CACHE STRING "Apple Developer ID Application"
)

# the string id of your apple developer id installer certificate
# something like "Developer ID Installer: firstname lastname (ABCDE12345)"
set(APPLE_DEVELOPER_ID_INSTALLER
  ""
  CACHE STRING "Apple Developer ID Installer"
)

# NOTARIZATION #################################################################

set(ENABLE_NOTARIZATION
  FALSE
  CACHE STRING "Flag to enable notarization"
)

# the profile name created with `xcrun notarytool store-credentials
# --apple-id "name@example.com" --team-id "ABCD123456"` if we did so.
# if left blank, apple developer id and apple team id vars below will be used
# and the final packaging script will prompt for an app-specific password you
# should have created on https://appleid.apple.com/account/manage
set(APPLE_NOTARIZATION_KEYCHAIN_PROFILE
  ""
  CACHE STRING "Apple Keychain Notarytool Profile"
)

#[[
# TODO : SUPPORT NO PROFILE IN KEYCHAIN
set(APPLE_DEVELOPER_ID
  ""
  CACHE STRING "Apple Developer Account ID (likely the associated email address)"
)

# todo: deduce the team id with a regexp from apple developer id application / installer
set(APPLE_TEAM_ID
  ""
  CACHE STRING "The code between parentheses after \"Apple Developer ID: Application/Installer (...)\""
)
#]]
