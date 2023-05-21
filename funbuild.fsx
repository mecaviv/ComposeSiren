#r "nuget: Fun.Result"
#r "nuget: Fun.Build, 0.3.8"

open Fun.Result
open Fun.Build

pipeline "Fun.Build" {
    description "building f# & rust code"
    stage "F#" {
        run "dotnet tool restore"
        run "dotnet restore"
        run "dotnet fable Source/fsharp/ComposeSiren.fsproj -o Source/generated/rust/mecaviv-siren-declarations-rs/src --lang rust"
    }
    stage "rust" {
        workingDir "./Source/rust"
        run "cargo build"
    }
    runIfOnlySpecified false
}

pipeline "run" {
    description "running the rust sample program"
    
    stage "run" {
        workingDir "./Source/rust"
        run "cargo run"
        
    }
    runIfOnlySpecified
}

tryPrintPipelineCommandHelp ()