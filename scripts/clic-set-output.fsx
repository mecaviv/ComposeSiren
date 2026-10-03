// Set clic.output_device over MCP (F#).
//   dotnet fsi /tmp/clic-set-output.fsx ["MacBook Air Speakers"|"Main output"|...]

open System
open System.IO
open System.Net.Http
open System.Text
open System.Text.Json.Nodes

let discoveryPath =
    Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.UserProfile), ".composesiren_mcp.json")

let http = new HttpClient(Timeout = TimeSpan.FromSeconds 5.0)

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
                            let p = t.Substring(5).Trim()
                            if p.Length > 0 then Some p else None
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
                    printfn "unparseable: %s" (body.Substring(0, min 200 body.Length))
                    return None
        }

    do
        task {
            let init =
                sprintf
                    """{"jsonrpc":"2.0","id":%d,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"clic-set-output","version":"0"}}}"""
                    (nextRpcId ())

            let! _ = post init false
            let! _ = post """{"jsonrpc":"2.0","method":"notifications/initialized"}""" true
            ()
        }
        |> Async.AwaitTask
        |> Async.RunSynchronously

    member _.CallTool(name: string, args: string) =
        let payload =
            sprintf
                """{"jsonrpc":"2.0","id":%d,"method":"tools/call","params":{"name":"%s","arguments":%s}}"""
                (nextRpcId ())
                name
                args

        post payload false

let toolText (node: JsonNode) =
    let result = node.["result"]

    if isNull result then
        node.ToJsonString()
    else
        let content = result.["content"]

        if isNull content then
            result.ToJsonString()
        else
            content.AsArray()
            |> Seq.map (fun c ->
                let t = c.["text"]
                if isNull t then c.ToJsonString() else t.GetValue<string>())
            |> String.concat "\n"

let main =
    task {
        let entries = JsonNode.Parse(File.ReadAllText discoveryPath)
        let e = entries.AsArray()[0]
        let port = e.["port"].GetValue<int>()
        let url = sprintf "http://127.0.0.1:%d/mcp" port
        printfn "MCP %s pid=%d" url (e.["pid"].GetValue<int>())

        let device =
            match fsi.CommandLineArgs with
            | [| _; d |] -> d
            | _ -> "MacBook Air Speakers"

        let mcp = Mcp url

        let! listed = mcp.CallTool("list_audio_devices", "{}")
        match listed with
        | Some n -> printfn "devices: %s" (toolText n)
        | None -> printfn "devices: (no body)"

        let! got = mcp.CallTool("get_setting", """{"id":"clic.output_device"}""")
        match got with
        | Some n -> printfn "before: %s" (toolText n)
        | None -> printfn "before: (no body)"

        let escaped =
            "\"" + device.Replace("\\", "\\\\").Replace("\"", "\\\"") + "\""

        let! setr = mcp.CallTool("set_setting", sprintf """{"id":"clic.output_device","value":%s}""" escaped)

        match setr with
        | Some n -> printfn "set: %s" (toolText n)
        | None -> printfn "set: (no body)"

        let! after = mcp.CallTool("get_setting", """{"id":"clic.output_device"}""")
        match after with
        | Some n -> printfn "after: %s" (toolText n)
        | None -> printfn "after: (no body)"
    }

main |> Async.AwaitTask |> Async.RunSynchronously
