# Collect concrete JUCE targets from logical names + formats.
# Usage:
#   juce_collect_targets(OUT_VAR "${PLUGIN_TARGETS}" "${FORMATS}")
# OUT_VAR will contain existing targets like OneSiren_Standalone;SirenOrchestra_VST3;...
function(juce_collect_targets out_var plugin_names formats)
    set(result "")
    foreach(name IN LISTS plugin_names)
        foreach(fmt IN LISTS formats)
            set(tgt "${name}_${fmt}")
            if(TARGET "${tgt}")
                list(APPEND result "${tgt}")
            endif()
        endforeach()
    endforeach()
    set(${out_var} "${result}" PARENT_SCOPE)
endfunction()