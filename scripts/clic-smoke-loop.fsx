// Stream channel-10 clicks into SirenOrchestra via its MCP send_midi.
// Even note = strong (fort), odd = weak (faible); velocity > 1.
//
//   dotnet fsi /tmp/clic-smoke-loop.fsx [intervalSeconds]

open System
open System.IO
open System.Net.Http
open System.Text
open System.Text.Json.Nodes

let status = 0x99uy // note-on, channel 10 (0-based 9)
let noteFort = 36uy // even -> strong
let noteFaible = 37uy // odd  -> weak
let vel = 100uy

let intervalSeconds =
    match fsi.CommandLineArgs with
    | [| _; s |] -> float s
    | _ -> 0.45

let discoveryPath =
    Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.UserProfile), ".composesiren_mcp.json")

let http = new HttpClient(Timeout = TimeSpan.FromSeconds 5.0)

let tryHealth (port: int) =
    task {
        try
            let! _ = http.GetStringAsync(sprintf "http://127.0.0.1:%d/health" port)
            return true
        with _ ->
            return false
    }

let waitForInstance () =
    task {
        let deadline = DateTime.UtcNow.AddSeconds 20.0
        let mutable found: (string * int * int) option = None

        while found.IsNone && DateTime.UtcNow < deadline do
            try
                let entries = JsonNode.Parse(File.ReadAllText discoveryPath)

                for e in entries.AsArray() do
                    if found.IsNone then
                        let port = e.["port"].GetValue<int>()
                        let pid = e.["pid"].GetValue<int>()
                        let! ok = tryHealth port

                        if ok then
                            found <- Some(sprintf "http://127.0.0.1:%d/mcp" port, port, pid)
            with _ ->
                ()

            if found.IsNone then
                do! Async.Sleep 300

        match found with
        | Some x -> return x
        | None -> return failwith "no running SirenOrchestra MCP instance found"
    }

type Mcp(url: string) =
    let mutable nextId = 0
    let mutable session: string = null

    let nextRpcId () =
        nextId <- nextId + 1
        nextId

    let post (payload: string) (notify: bool) =
        task {
            use content = new StringContent(payload, Encoding.UTF8, "application/json")
            use req = new HttpRequestMessage(HttpMethod.Post, url)
            req.Content <- content
            req.Headers.Accept.ParseAdd "application/json"
            req.Headers.Accept.ParseAdd "text/event-stream"

            if not (isNull session) then
                req.Headers.TryAddWithoutValidation("Mcp-Session-Id", session) |> ignore

            let! resp = http.SendAsync req

            let sid =
                match resp.Headers.TryGetValues "Mcp-Session-Id" with
                | true, vals -> Seq.head vals
                | _ ->
                    match resp.Headers.TryGetValues "mcp-session-id" with
                    | true, vals -> Seq.head vals
                    | _ -> null

            if not (isNull sid) then
                session <- sid

            let! body = resp.Content.ReadAsStringAsync()

            if notify || String.IsNullOrWhiteSpace body then
                return None
            else
                let candidates =
                    body.Split '\n'
                    |> Array.choose (fun line ->
                        let t = line.TrimStart()

                        if t.StartsWith "data:" then
                            let payload = t.Substring(5).Trim()
                            if payload.Length > 0 then Some payload else None
                        else
                            None)

                let json =
                    candidates
                    |> Array.tryFind (fun s ->
                        s.Contains "\"jsonrpc\"" || s.Contains "\"result\"" || s.Contains "\"error\"")
                    |> Option.defaultValue (
                        if candidates.Length > 0 then candidates[candidates.Length - 1] else body)

                try
                    return Some(JsonNode.Parse json)
                with _ ->
                    printfn "unparseable response: %s" (body.Substring(0, min 240 body.Length))
                    return None
        }

    do
        task {
            let init =
                sprintf
                    """{"jsonrpc":"2.0","id":%d,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"clic-smoke","version":"0"}}}"""
                    (nextRpcId ())

            let! _ = post init false
            let! _ = post """{"jsonrpc":"2.0","method":"notifications/initialized"}""" true
            ()
        }
        |> Async.AwaitTask
        |> Async.RunSynchronously

    member _.SendMidi(status: byte, data1: byte, data2: byte) =
        let payload =
            sprintf
                """{"jsonrpc":"2.0","id":%d,"method":"tools/call","params":{"name":"send_midi","arguments":{"status":%d,"data1":%d,"data2":%d}}}"""
                (nextRpcId ())
                (int status)
                (int data1)
                (int data2)

        post payload false

let main =
    task {
        let! url, port, pid = waitForInstance ()
        printfn "clic smoke -> %s  port=%d  pid=%d" url port pid

        let mcp = Mcp url
        let sw = Diagnostics.Stopwatch.StartNew()
        let mutable n = 0

        while true do
            let isFort = n % 2 = 0
            let note = if isFort then noteFort else noteFaible
            let kind = if isFort then "fort" else "faible"
            let! reply = mcp.SendMidi(status, note, vel)

            let ok =
                match reply with
                | Some node ->
                    let text = node.ToJsonString()

                    if text.Contains "\"ok\": false" || text.Contains "\"ok\":false" then
                        printfn "send failed: %s" (text.Substring(0, min 200 text.Length))
                        false
                    else
                        true
                | None -> true

            if ok then
                printfn "click %04d  %-6s  note=%d  vel=%d  t=%.2fs" n kind (int note) (int vel) sw.Elapsed.TotalSeconds

            n <- n + 1
            do! Async.Sleep(int (intervalSeconds * 1000.0))
    }

main |> Async.AwaitTask |> Async.RunSynchronously
