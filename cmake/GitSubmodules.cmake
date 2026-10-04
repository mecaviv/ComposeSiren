# ------------------------------------------------------------------------------
# composesiren_ensure_git_submodules: initialises the git submodules the build
# needs (Dependencies/JUCE) when they are not checked out yet, for example after
# a clone without --recursive, so CMake (or an IDE like CLion) configures from a
# plain clone.
#
#   option COMPOSESIREN_GIT_SUBMODULE (ON): run
#     git submodule update --init --recursive
#   when a submodule listed in .gitmodules is missing. Off: only check, and stop
#   with the command to run.
#
# Cheap when everything is there: it only looks at files, git does not run.
# A submodule counts as present when its marker exists: <path>/CMakeLists.txt
# for the ones added with add_subdirectory (JUCE), <path>/.git otherwise.

include_guard(GLOBAL)

option(COMPOSESIREN_GIT_SUBMODULE
       "Initialise missing git submodules (Dependencies/JUCE) at configure time"
       ON)

function(_composesiren_submodule_paths out_var)
  set(gitmodules "${CMAKE_SOURCE_DIR}/.gitmodules")
  set(paths "")
  if(EXISTS "${gitmodules}")
    file(STRINGS "${gitmodules}" lines REGEX "^[ \t]*path[ \t]*=")
    foreach(line IN LISTS lines)
      string(REGEX REPLACE "^[ \t]*path[ \t]*=[ \t]*" "" path "${line}")
      string(STRIP "${path}" path)
      list(APPEND paths "${path}")
    endforeach()
  endif()
  set(${out_var} "${paths}" PARENT_SCOPE)
endfunction()

function(_composesiren_missing_submodules out_var)
  _composesiren_submodule_paths(paths)
  set(missing "")
  foreach(path IN LISTS paths)
    if(path STREQUAL "Dependencies/JUCE")
      set(marker "${CMAKE_SOURCE_DIR}/${path}/CMakeLists.txt")
    else()
      set(marker "${CMAKE_SOURCE_DIR}/${path}/.git")
    endif()
    if(NOT EXISTS "${marker}")
      list(APPEND missing "${path}")
    endif()
  endforeach()
  set(${out_var} "${missing}" PARENT_SCOPE)
endfunction()

function(composesiren_ensure_git_submodules)
  _composesiren_missing_submodules(missing)
  if(NOT missing)
    return()
  endif()

  set(hint "Run in ${CMAKE_SOURCE_DIR}:\n  git submodule update --init --recursive")
  if(NOT COMPOSESIREN_GIT_SUBMODULE)
    message(FATAL_ERROR
      "Missing git submodules: ${missing} (COMPOSESIREN_GIT_SUBMODULE is OFF).\n${hint}")
  endif()
  if(NOT EXISTS "${CMAKE_SOURCE_DIR}/.git")
    message(FATAL_ERROR
      "Missing dependencies: ${missing}, and ${CMAKE_SOURCE_DIR} is not a git "
      "checkout, so they cannot be fetched as submodules. Clone the repository "
      "with git (git clone --recursive), or put them at those paths.")
  endif()
  find_package(Git QUIET)
  if(NOT GIT_FOUND)
    message(FATAL_ERROR "Missing git submodules: ${missing}, and git was not found.\n${hint}")
  endif()

  message(STATUS "Initialising git submodules (${missing}): git submodule update --init --recursive")
  execute_process(
    COMMAND "${GIT_EXECUTABLE}" submodule update --init --recursive
    WORKING_DIRECTORY "${CMAKE_SOURCE_DIR}"
    RESULT_VARIABLE result
    ERROR_VARIABLE error
  )
  if(NOT result EQUAL 0)
    message(FATAL_ERROR
      "git submodule update --init --recursive failed (${result}):\n${error}\n"
      "Check the network and the access to the submodule URLs in .gitmodules, then:\n${hint}")
  endif()

  _composesiren_missing_submodules(missing)
  if(missing)
    message(FATAL_ERROR
      "Git submodules still missing after git submodule update: ${missing}.\n${hint}")
  endif()
  message(STATUS "Initialising git submodules - done")
endfunction()
