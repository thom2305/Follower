// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Thom (thom2305)

using Follower.Data;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Media.Imaging;
using Microsoft.UI.Xaml.Navigation;
using System.Runtime.InteropServices.WindowsRuntime;
using Windows.Storage;
using Windows.Storage.Pickers;
using Windows.Storage.Streams;

namespace Follower;

public sealed partial class ArchiveDetailPage : Page
{
    public static event Action<long>? Deleted;

    ArchiveMsg msg = null!;

    public ArchiveDetailPage()
    {
        InitializeComponent();
        Ui.Fit(Scroll, Root);
    }

    protected override void OnNavigatedTo(NavigationEventArgs e)
    {
        msg = (ArchiveMsg)e.Parameter;
        var zone = AppState.Config.Zone;

        var head = new StackPanel { Orientation = Orientation.Horizontal, Spacing = 12 };
        head.Children.Add(new PersonPicture { Width = 48, Height = 48, DisplayName = msg.AuthorName, ProfilePicture = Ui.Img(msg.AuthorAvatar, 96) });
        head.Children.Add(new TextBlock { Text = msg.AuthorName, VerticalAlignment = VerticalAlignment.Center, Style = (Style)Application.Current.Resources["TitleTextBlockStyle"] });
        Root.Children.Add(head);

        Root.Children.Add(new TextBlock
        {
            Text = string.IsNullOrWhiteSpace(msg.Content) ? "(no text)" : msg.Content!.Trim(),
            TextWrapping = TextWrapping.Wrap, IsTextSelectionEnabled = true,
        });
        Root.Children.Add(Ui.Muted($"Sent {msg.MessageTs.Pretty(zone)}"));
        Root.Children.Add(Ui.Muted($"Saved {msg.SavedAt.Pretty(zone)}"));

        if (msg.Attachments.Count > 0)
        {
            var card = new Border { Style = (Style)Application.Current.Resources["CardBorder"] };
            var panel = new StackPanel { Spacing = 10 };
            panel.Children.Add(Ui.Heading("Attachments"));
            foreach (var a in msg.Attachments) panel.Children.Add(AttachmentRow(a));
            card.Child = panel;
            Root.Children.Add(card);
        }

        if (!string.IsNullOrEmpty(msg.JumpUrl))
        {
            var open = new Button { Content = "Open in Discord" };
            open.Click += async (_, _) => await OpenAsync(msg.JumpUrl!);
            Root.Children.Add(open);
        }

        var del = new Button { Content = "Delete" };
        del.Click += Delete_Click;
        Root.Children.Add(del);
        Root.Children.Add(Ui.Muted($"ID {msg.MessageId}"));
    }

    static async Task OpenAsync(string url)
    {
        if (Uri.TryCreate(url, UriKind.Absolute, out var u)) await Windows.System.Launcher.LaunchUriAsync(u);
    }

    UIElement AttachmentRow(Attachment a)
    {
        var isImage = a.ContentType?.StartsWith("image/") == true;
        var where = !a.Stored ? "link only" : a.Compressed ? $"stored (compressed to {Util.HumanSize(a.StoredSize)})" : "stored";
        var info = $"{Util.HumanSize(a.Size)} · {a.ContentType ?? "unknown"} · {where}";

        var outer = new StackPanel { Spacing = 8 };
        var grid = new Grid { ColumnSpacing = 8 };
        grid.ColumnDefinitions.Add(new ColumnDefinition { Width = new GridLength(1, GridUnitType.Star) });
        grid.ColumnDefinitions.Add(new ColumnDefinition { Width = GridLength.Auto });

        var labels = new StackPanel();
        labels.Children.Add(new TextBlock { Text = a.Filename, TextWrapping = TextWrapping.Wrap, IsTextSelectionEnabled = true });
        labels.Children.Add(Ui.Muted(info));
        grid.Children.Add(labels);

        var buttons = new StackPanel { Orientation = Orientation.Horizontal, Spacing = 8, VerticalAlignment = VerticalAlignment.Center };
        Grid.SetColumn(buttons, 1);
        if (a.Stored)
        {
            if (isImage)
            {
                var preview = new Button { Content = "Preview" };
                preview.Click += async (_, _) => await PreviewAsync(a, preview, outer);
                buttons.Children.Add(preview);
            }
            var save = new Button { Content = "Save…" };
            save.Click += async (_, _) => await SaveAsync(a);
            buttons.Children.Add(save);
        }
        else if (!string.IsNullOrEmpty(a.Url))
        {
            var open = new Button { Content = "Open original" };
            open.Click += async (_, _) => await OpenAsync(a.Url!);
            buttons.Children.Add(open);
        }
        grid.Children.Add(buttons);
        outer.Children.Add(grid);
        return outer;
    }

    async Task PreviewAsync(Attachment a, Button button, StackPanel host)
    {
        button.IsEnabled = false;
        byte[]? bytes = null;
        var ok = await AppState.RunAsync(async () => bytes = await AppState.Repo.LoadAttachmentAsync(a.Id));
        if (!ok || bytes is null) { button.IsEnabled = true; return; }
        try
        {
            using var ms = new InMemoryRandomAccessStream();
            await ms.WriteAsync(bytes.AsBuffer());
            ms.Seek(0);
            var bmp = new BitmapImage();
            await bmp.SetSourceAsync(ms);
            host.Children.Add(new Image { Source = bmp, MaxHeight = 360, Stretch = Microsoft.UI.Xaml.Media.Stretch.Uniform, HorizontalAlignment = HorizontalAlignment.Left });
        }
        catch { AppState.Say("Couldn't preview that image."); button.IsEnabled = true; }
    }

    async Task SaveAsync(Attachment a)
    {
        var picker = new FileSavePicker { SuggestedFileName = Path.GetFileNameWithoutExtension(a.Filename) };
        var ext = Path.GetExtension(a.Filename);
        picker.FileTypeChoices.Add(ext.Length > 1 ? ext.TrimStart('.').ToUpperInvariant() + " file" : "File", new List<string> { ext.Length > 1 ? ext : ".bin" });
        WinRT.Interop.InitializeWithWindow.Initialize(picker, WinRT.Interop.WindowNative.GetWindowHandle(App.MainWin));
        var file = await picker.PickSaveFileAsync();
        if (file is null) return;
        var ok = await AppState.RunAsync(async () =>
        {
            var bytes = await AppState.Repo.LoadAttachmentAsync(a.Id);
            await FileIO.WriteBytesAsync(file, bytes);
        });
        if (ok) AppState.Say("Saved");
    }

    async void Delete_Click(object sender, RoutedEventArgs e)
    {
        if (!await Ui.ConfirmAsync(XamlRoot, "Delete this message?", "This also deletes the stored attachments.", "Delete")) return;
        var ok = await AppState.RunAsync(() => AppState.Repo.ArchiveDeleteAsync(msg.MessageId));
        if (!ok) return;
        Deleted?.Invoke(msg.MessageId);
        AppState.Say("Deleted (including stored attachments).");
        if (Frame.CanGoBack) Frame.GoBack();
    }
}
