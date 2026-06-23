# Resources preprocessing (python)

# Paths
# we already define these in Config.cmake
#set(RES_INPUT_DIR "${CMAKE_SOURCE_DIR}/Resources")
#set(RES_OUTPUT_DIR "${CMAKE_BINARY_DIR}/Resources-fixed")
set(RES_INPUT_DIR "${ORIGINAL_RESOURCES_DIR}")
set(RES_OUTPUT_DIR "${PROCESSED_RESOURCES_DIR}")

# we want to use the scripts in the current directory
set(RES_SCRIPT "${CMAKE_CURRENT_LIST_DIR}/patko_fix_data/fix_data.py")

# Locate Python
find_package(Python3 COMPONENTS Interpreter REQUIRED)
#set(PYTHON_EXECUTABLE "${Python3_EXECUTABLE}")

# Build-local virtualenv
if(WIN32)
    set(PYENV_PY "${CMAKE_BINARY_DIR}/pyenv/Scripts/python.exe")
    set(PYENV_PIP "${CMAKE_BINARY_DIR}/pyenv/Scripts/pip.exe")
    set(PYENV_BIN "${CMAKE_BINARY_DIR}/pyenv/Scripts")
    set(MAKE_VENV_CMD "${Python3_EXECUTABLE}" "-m" "venv" "${CMAKE_BINARY_DIR}/pyenv")
else()
    set(PYENV_PY "${CMAKE_BINARY_DIR}/pyenv/bin/python")
    set(PYENV_PIP "${CMAKE_BINARY_DIR}/pyenv/bin/pip")
    set(PYENV_BIN "${CMAKE_BINARY_DIR}/pyenv/bin")
    set(MAKE_VENV_CMD "${Python3_EXECUTABLE}" "-m" "venv" "${CMAKE_BINARY_DIR}/pyenv")
endif()

set(REQUIREMENTS_TXT "${CMAKE_CURRENT_LIST_DIR}/requirements.txt")
set(VENV_STAMP "${CMAKE_BINARY_DIR}/pyenv/.venv.ready")

add_custom_command(
        OUTPUT "${VENV_STAMP}"
        COMMAND ${CMAKE_COMMAND} -E make_directory "${CMAKE_BINARY_DIR}/pyenv"
        COMMAND ${MAKE_VENV_CMD}
        COMMAND "${PYENV_PY}" -m pip install --upgrade pip
        COMMAND "${PYENV_PIP}" install --upgrade -r "${REQUIREMENTS_TXT}"
        COMMAND ${CMAKE_COMMAND} -E touch "${VENV_STAMP}"
        DEPENDS "${REQUIREMENTS_TXT}"
        COMMENT "Preparing Python virtualenv and installing requirements"
        VERBATIM
)

add_custom_target(pyenv_ready
    DEPENDS "${VENV_STAMP}"
)

# Discover inputs (configure-time listing; rerun cmake if files are added/removed)
file(GLOB RES_INPUT_FILES
        "${RES_INPUT_DIR}/dataAmpS*"
        "${RES_INPUT_DIR}/dataFreqS*"
        "${RES_INPUT_DIR}/datadureTabsS*"
        "${RES_INPUT_DIR}/dataVectorInterval*")

# Use a stamp file to represent all generated outputs
set(RES_STAMP "${RES_OUTPUT_DIR}/.generated.stamp")

add_custom_command(
        OUTPUT "${RES_STAMP}"
        COMMAND ${CMAKE_COMMAND} -E make_directory "${RES_OUTPUT_DIR}"
        COMMAND ${CMAKE_COMMAND} -E env PYTHONUTF8=1
        "${PYENV_PY}" "${RES_SCRIPT}"
        --resources "${RES_INPUT_DIR}"
        --output "${RES_OUTPUT_DIR}"
        # Touch stamp so cmake sees a concrete output updated after script completes
        COMMAND ${CMAKE_COMMAND} -E touch "${RES_STAMP}"
        DEPENDS "${VENV_STAMP}" "${RES_SCRIPT}" ${RES_INPUT_FILES}
        COMMENT "Preprocessing Resources (non-destructive) -> ${RES_OUTPUT_DIR}"
        VERBATIM
)

# Public target to (re)generate processed resources
add_custom_target(run_py_resources
        DEPENDS "${RES_STAMP}"
)

foreach(PLUGIN_TARGET IN LISTS PLUGIN_TARGETS)
    foreach(FORMAT IN LISTS FORMATS)
        set(target "${PLUGIN_TARGET}_${FORMAT}")
        if(TARGET "${target}")
            # message(STATUS "adding run_py_resources dependency to ${target}")
            add_dependencies("${target}" run_py_resources)
        endif()
    endforeach()
endforeach()

# Export the interpreter path for parent scripts if needed
#set(PROCESSING_PYTHON "${PYENV_PY}" PARENT_SCOPE)
#set(PROCESSED_RESOURCES_DIR "${RES_OUTPUT_DIR}" PARENT_SCOPE)
#set(RAW_RESOURCES_DIR "${RES_INPUT_DIR}" PARENT_SCOPE)