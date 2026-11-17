// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Thom (thom2305)

using System.Collections.ObjectModel;
using Follower.Data;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;

namespace Follower;

public sealed partial class LookupPage : Page
{
    readonly ObservableCollection<Row> rows = new();
    List<LookupEntry> results = new();

    public LookupPage()
    {
        InitializeComponent();
        Ui.Fit(Scroll, Column);
        NavigationCacheMode = Microsoft.UI.Xaml.Navigation.NavigationCacheMode.Required;
        ResultsList.ItemsSource = rows;
        RelationBox.Items.Add("Don't set");
        foreach (var s in Util.Statuses) RelationBox.Items.Add(s.Label);
        RelationBox.SelectedIndex = 0;
    }

    void IdsBox_TextChanged(object sender, TextChangedEventArgs e) =>
        HintText.Text = $"{Util.ParseIds(IdsBox.Text).Count} ID(s) found · paste as many as you like (max 25)";

    void SaveSwitch_Toggled(object sender, RoutedEventArgs e) =>
        SaveOptions.Visibility = SaveSwitch.IsOn ? Visibility.Visible : Visibility.Collapsed;

    void Render()
    {
        rows.Clear();
        foreach (var r in results)
        {
            var name = r.Profile?.DisplayName ?? "Not found";
            var sub = (r.Profile is { } p ? $"@{p.Username} · " : "") + r.UserId + "\n"
                + (r.Person is { } per ? "In your list: " + Util.StatusLabel(per.Relation) : "Not tracked");
            foreach (var c in r.Changes) sub += $"\n{c.Key}: {Ui.Trunc(c.From, 60)} → {Ui.Trunc(c.To, 60)}";
            rows.Add(new Row
            {
                Id = r.UserId, Name = name, Subtitle = sub, Avatar = Ui.Img(r.Profile?.Avatar, 80),
                SaveVisibility = r.Person is null ? Visibility.Visible : Visibility.Collapsed,
            });
        }
        ResultsTitle.Visibility = rows.Count > 0 ? Visibility.Visible : Visibility.Collapsed;
        var untracked = results.Count(r => r.Person is null);
        SaveAllButton.Visibility = untracked > 1 ? Visibility.Visible : Visibility.Collapsed;
        SaveAllButton.Content = $"Save all {untracked} to list";
    }

    async void Go_Click(object sender, RoutedEventArgs e)
    {
        var ids = Util.ParseIds(IdsBox.Text).Take(25).ToList();
        if (ids.Count == 0) { AppState.Say("Paste at least one Discord ID first."); return; }
        var save = SaveSwitch.IsOn;
        string? relation = save && RelationBox.SelectedIndex > 0 ? Util.Statuses[RelationBox.SelectedIndex - 1].Key : null;
        var tag = Util.NormTag(TagBox.Text);
        string? tagArg = save && tag.Length > 0 ? tag : null;

        GoButton.IsEnabled = false; GoButton.Content = "Looking up…";
        var ok = await AppState.RunAsync(async () => results = await AppState.Repo.LookupAsync(ids, save, relation, tagArg));
        GoButton.IsEnabled = true; GoButton.Content = "Look up";
        if (!ok) return;
        Render();
        if (save) _ = AppState.LoadPeopleAsync();
    }

    async Task SaveIdsAsync(List<long> ids)
    {
        if (ids.Count == 0) return;
        var ok = await AppState.RunAsync(async () =>
        {
            await AppState.Repo.SaveManyAsync(ids);
            foreach (var id in ids)
            {
                var p = await AppState.Repo.GetPersonAsync(id);
                var entry = results.FirstOrDefault(x => x.UserId == id);
                if (entry is not null) entry.Person = p;
            }
        });
        if (!ok) return;
        Render();
        _ = AppState.LoadPeopleAsync();
        AppState.Say($"Saved {ids.Count} to your list");
    }

    async void SaveOne_Click(object sender, RoutedEventArgs e)
    {
        if (sender is FrameworkElement { Tag: long id }) await SaveIdsAsync(new List<long> { id });
    }

    async void SaveAll_Click(object sender, RoutedEventArgs e) =>
        await SaveIdsAsync(results.Where(r => r.Person is null).Select(r => r.UserId).ToList());

    void Results_ItemClick(object sender, ItemClickEventArgs e)
    {
        if (e.ClickedItem is Row r) Frame.Navigate(typeof(PersonPage), r.Id);
    }
}
