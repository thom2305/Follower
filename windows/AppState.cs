// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Thom (thom2305)

using Follower.Data;

namespace Follower;

public static class AppState
{
    static readonly ConfigStore store = new();
    static Config config = store.Load();

    public static Config Config => config;
    public static Repo Repo { get; } = new(new Api(() => config));

    public static List<PersonEntry> People { get; private set; } = new();
    public static List<TagCount> Tags { get; private set; } = new();
    public static bool Loading { get; private set; }
    public static string? Progress { get; private set; }

    public static event Action? Changed;
    public static Action<string>? Toast;

    public static void Say(string msg) => Toast?.Invoke(msg);

    public static async Task<bool> RunAsync(Func<Task> work)
    {
        try { await work(); return true; }
        catch (OperationCanceledException) { return false; }
        catch (Exception e) { Say(string.IsNullOrWhiteSpace(e.Message) ? "Something went wrong" : e.Message); return false; }
    }

    public static async Task SaveConfigAsync(Config c, bool test)
    {
        c = new Config(c.Url.Trim(), c.Token.Trim(), c.Timezone.Trim());
        try { store.Save(c); }
        catch (Exception e) { Say("Couldn't save settings: " + e.Message); return; }
        config = c;
        if (!test) { Say("Saved"); return; }
        await RunAsync(async () =>
        {
            var n = await Repo.CountsAsync();
            Say($"Connected: {n.People} people, {n.Archive} archived messages");
            await LoadPeopleAsync();
        });
    }

    public static Task LoadPeopleAsync() => RunAsync(LoadPeopleNowAsync);

    static async Task LoadPeopleNowAsync()
    {
        Loading = true; Changed?.Invoke();
        try
        {
            People = await Repo.ListPeopleAsync();
            Tags = await Repo.AllTagsAsync();
        }
        finally { Loading = false; Changed?.Invoke(); }
    }

    public static async Task RefreshAllAsync(IReadOnlyList<long> ids)
    {
        if (ids.Count == 0 || Progress is not null) return;
        int changed = 0, gone = 0;
        var ok = await RunAsync(async () =>
        {
            for (var i = 0; i < ids.Count; i++)
            {
                Progress = $"Refreshing {i + 1}/{ids.Count}…"; Changed?.Invoke();
                var r = await Repo.TakeSnapshotAsync(ids[i]);
                if (r.Profile is null) gone++; else if (r.Changes.Count > 0) changed++;
            }
        });
        Progress = null; Changed?.Invoke();
        if (!ok) return;
        await LoadPeopleAsync();
        Say($"Refreshed {ids.Count} · {changed} changed" + (gone > 0 ? $" · {gone} not found" : ""));
    }
}
