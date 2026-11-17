// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Thom (thom2305)

using System.Collections.ObjectModel;
using Follower.Data;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Windows.System;

namespace Follower;

public sealed partial class ArchivePage : Page
{
    readonly ObservableCollection<Row> rows = new();
    List<AuthorChoice> authors = new();
    bool authorsLoaded;
    bool syncing;

    public ArchivePage()
    {
        InitializeComponent();
        Ui.Fit(Scroll, Column);
        NavigationCacheMode = Microsoft.UI.Xaml.Navigation.NavigationCacheMode.Required;
        ResultsList.ItemsSource = rows;
        PickBox.Items.Add("Anyone");
        PickBox.SelectedIndex = 0;
        ArchiveDetailPage.Deleted += id =>
        {
            var r = rows.FirstOrDefault(x => x.Id == id);
            if (r is not null) rows.Remove(r);
            UpdateTitle();
        };
        Loaded += async (_, _) => await LoadAuthorsAsync();
    }

    async Task LoadAuthorsAsync()
    {
        if (authorsLoaded || !AppState.Config.Ready) return;
        authorsLoaded = true;
        var ok = await AppState.RunAsync(async () => authors = await AppState.Repo.AuthorsAsync());
        if (!ok) { authorsLoaded = false; return; }
        syncing = true;
        PickBox.Items.Clear();
        PickBox.Items.Add("Anyone");
        foreach (var a in authors) PickBox.Items.Add($"{a.Name} ({a.Count})");
        PickBox.SelectedIndex = 0;
        syncing = false;
    }

    void Pick_SelectionChanged(object sender, SelectionChangedEventArgs e)
    {
        if (syncing) return;
        var i = PickBox.SelectedIndex;
        if (i == 0) AuthorBox.Text = "";
        else if (i > 0 && i - 1 < authors.Count) AuthorBox.Text = authors[i - 1].Id.ToString();
    }

    void UpdateTitle()
    {
        ResultsTitle.Visibility = Visibility.Visible;
        ResultsTitle.Text = rows.Count == 0 ? "Nothing found." : $"{rows.Count} result(s)";
    }

    async void Search_Click(object sender, RoutedEventArgs e) => await SearchAsync();

    async void Box_KeyDown(object sender, Microsoft.UI.Xaml.Input.KeyRoutedEventArgs e)
    {
        if (e.Key != VirtualKey.Enter) return;
        e.Handled = true;
        await SearchAsync();
    }

    async Task SearchAsync()
    {
        var zone = AppState.Config.Zone;
        List<ArchiveMsg> msgs = new();
        SearchButton.IsEnabled = false;
        var ok = await AppState.RunAsync(async () => msgs = await AppState.Repo.ArchiveSearchAsync(TextBox.Text.Trim(), AuthorBox.Text.Trim()));
        SearchButton.IsEnabled = true;
        if (!ok) return;
        rows.Clear();
        foreach (var m in msgs)
        {
            var body = string.IsNullOrWhiteSpace(m.Content) ? "(no text)" : m.Content!.Trim();
            var sub = Ui.Trunc(body, 240) + (m.Attachments.Count > 0 ? $"\n{m.Attachments.Count} attachment(s)" : "");
            rows.Add(new Row { Id = m.MessageId, Name = $"{m.AuthorName} · {m.MessageTs.Pretty(zone)}", Subtitle = sub, Avatar = Ui.Img(m.AuthorAvatar, 64), Data = m });
        }
        UpdateTitle();
    }

    async void Random_Click(object sender, RoutedEventArgs e)
    {
        ArchiveMsg? m = null;
        var ok = await AppState.RunAsync(async () => m = await AppState.Repo.ArchiveRandomAsync(AuthorBox.Text.Trim()));
        if (!ok) return;
        if (m is null) AppState.Say("Archive is empty.");
        else Frame.Navigate(typeof(ArchiveDetailPage), m);
    }

    void Results_ItemClick(object sender, ItemClickEventArgs e)
    {
        if (e.ClickedItem is Row { Data: ArchiveMsg m }) Frame.Navigate(typeof(ArchiveDetailPage), m);
    }
}
