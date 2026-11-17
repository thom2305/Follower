// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Thom (thom2305)

using System.Net.Http.Headers;
using System.Text;
using System.Text.Json.Nodes;

namespace Follower.Data;

public sealed class ApiException(string message) : Exception(message);
public sealed class Api
{
    readonly HttpClient http = new() { Timeout = TimeSpan.FromSeconds(60) };
    readonly Func<Config> config;

    public Api(Func<Config> config) => this.config = config;

    public Task<JsonObject> GetAsync(string path) => JsonAsync(HttpMethod.Get, path, null);
    public Task<JsonObject> PostAsync(string path, JsonObject? body = null) => JsonAsync(HttpMethod.Post, path, body ?? new JsonObject());
    public Task<JsonObject> DeleteAsync(string path) => JsonAsync(HttpMethod.Delete, path, null);

    async Task<JsonObject> JsonAsync(HttpMethod method, string path, JsonObject? body)
    {
        var data = await BytesAsync(method, path, body);
        try { if (JsonNode.Parse(data) is JsonObject o) return o; } catch { /* fall through */ }
        throw new ApiException("The server sent something that isn't JSON.");
    }

    public async Task<byte[]> BytesAsync(HttpMethod method, string path, JsonObject? body = null)
    {
        var c = config();
        if (!c.Ready) throw new ApiException("Set the server URL and API token in Settings first.");
        var baseUrl = c.Url.Trim().TrimEnd('/');
        if (!baseUrl.StartsWith("http://") && !baseUrl.StartsWith("https://")) baseUrl = "https://" + baseUrl;
        if (!Uri.TryCreate(baseUrl + path, UriKind.Absolute, out var uri))
            throw new ApiException("That server URL doesn't look valid.");

        using var req = new HttpRequestMessage(method, uri);
        req.Headers.Authorization = new AuthenticationHeaderValue("Bearer", c.Token.Trim());
        req.Headers.Accept.Add(new MediaTypeWithQualityHeaderValue("application/json"));
        if (body is not null)
            req.Content = new StringContent(body.ToJsonString(), Encoding.UTF8, "application/json");

        HttpResponseMessage resp;
        try { resp = await http.SendAsync(req); }
        catch (TaskCanceledException) { throw new ApiException("The server took too long to answer."); }
        catch (HttpRequestException e) { throw new ApiException("Couldn't reach the server: " + e.Message); }

        using (resp)
        {
            var bytes = await resp.Content.ReadAsByteArrayAsync();
            if (resp.IsSuccessStatusCode) return bytes;

            string? msg = null;
            try { msg = JsonNode.Parse(bytes).Str("error"); } catch { }
            var code = (int)resp.StatusCode;
            if (code == 401) throw new ApiException("Wrong API token (401).");
            throw new ApiException(msg ?? $"Server returned HTTP {code}.");
        }
    }
}
