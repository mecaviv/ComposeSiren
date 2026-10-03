// Capture a macOS window to PNG by owner-name substring (F#).
//   dotnet fsi scripts/capture-window.fsx [nameSubstring] [out.png]
// Platform ops live in mecaviv-dev-root/resources/fsx/Mecaviv.Platform.fsx.

#load "/Users/gauthiersegay/dev/src/github.com/mecaviv/mecaviv-dev-root/resources/fsx/Mecaviv.Platform.fsx"

open System
open System.IO
open Mecaviv.Platform

let nameSub =
    match fsi.CommandLineArgs with
    | [| _; n |]
    | [| _; n; _ |] -> n
    | _ -> "SirenOrchestra"

let outPath =
    match fsi.CommandLineArgs with
    | [| _; _; o |] -> o
    | [| _; o |] when o.EndsWith(".png", StringComparison.OrdinalIgnoreCase) -> o
    | _ -> "/tmp/sirenorchestra-clic-ui.png"

let wins = listWindows ()
printfn "windows: %d" wins.Length

let matches =
    wins
    |> List.filter (fun (_, owner, _, _, _, w, h) ->
        owner.IndexOf(nameSub, StringComparison.OrdinalIgnoreCase) >= 0
        && w > 50.0
        && h > 50.0)

match matches with
| [] ->
    eprintfn "no on-screen window matching %s" nameSub
    for (id, owner, _, x, y, w, h) in wins |> List.truncate 25 do
        eprintfn "  %d  %s  (%.0f,%.0f %.0fx%.0f)" id owner x y w h
    exit 1
| (id, owner, _, x, y, w, h) :: _ ->
    printfn "capturing %s  id=%d  (%.0f,%.0f %.0fx%.0f) -> %s" owner id x y w h outPath
    let code, _, err = captureWindow id outPath
    if code <> 0 then
        eprintfn "screencapture failed: %s" err
        exit code
    let fi = FileInfo outPath
    printfn "ok  %s  %d bytes" outPath fi.Length
