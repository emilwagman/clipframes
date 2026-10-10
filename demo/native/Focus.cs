using System; using System.IO; using System.Windows; using System.Windows.Markup; using System.Reflection;
[assembly: AssemblyTitle("Focus")] [assembly: AssemblyProduct("Focus")] [assembly: AssemblyDescription("Focus")] [assembly: AssemblyFileVersion("1.0.0.0")]
// A small native Windows app (WPF) to record Clipframes against: a focus timer. Nothing in it is a web page.
static class Program {
  const string Xaml = @"<Window xmlns=""http://schemas.microsoft.com/winfx/2006/xaml/presentation"" xmlns:x=""http://schemas.microsoft.com/winfx/2006/xaml""
        Title=""Focus"" Left=""0"" Top=""0"" Width=""1544"" Height=""1384"" Background=""#FAFAF8"" FontFamily=""Segoe UI Variable Display, Segoe UI"" WindowStartupLocation=""Manual"">
  <Window.Resources>
    <Style TargetType=""Button"">
      <Setter Property=""FontSize"" Value=""19""/><Setter Property=""FontWeight"" Value=""SemiBold""/><Setter Property=""Padding"" Value=""34,15""/>
      <Setter Property=""Foreground"" Value=""#17181C""/><Setter Property=""Background"" Value=""White""/><Setter Property=""BorderBrush"" Value=""#DADBE0""/>
      <Setter Property=""Template""><Setter.Value><ControlTemplate TargetType=""Button"">
        <Border Background=""{TemplateBinding Background}"" BorderBrush=""{TemplateBinding BorderBrush}"" BorderThickness=""1"" CornerRadius=""12"" Padding=""{TemplateBinding Padding}""><ContentPresenter HorizontalAlignment=""Center""/></Border>
      </ControlTemplate></Setter.Value></Setter>
    </Style>
  </Window.Resources>
  <Grid Margin=""96,72,96,72"">
    <Grid.ColumnDefinitions><ColumnDefinition Width=""*""/><ColumnDefinition Width=""430""/></Grid.ColumnDefinitions>
    <StackPanel VerticalAlignment=""Center"" HorizontalAlignment=""Center"">
      <TextBlock x:Name=""SessionLabel"" Text=""Deep work"" FontSize=""22"" Foreground=""#6B6E78"" HorizontalAlignment=""Center""/>
      <TextBlock x:Name=""TimeLeft"" Text=""25:00"" FontSize=""230"" FontWeight=""SemiBold"" Foreground=""#17181C"" HorizontalAlignment=""Center"" Margin=""0,6,0,22""/>
      <StackPanel Orientation=""Horizontal"" HorizontalAlignment=""Center"">
        <Button x:Name=""StartButton"" Content=""Start"" Background=""#3B3FD8"" BorderBrush=""#3B3FD8"" Foreground=""White"" Margin=""0,0,14,0""/>
        <Button x:Name=""ResetButton"" Content=""Reset"" Margin=""0,0,14,0""/>
        <Button x:Name=""SkipButton"" Content=""Skip break""/>
      </StackPanel>
      <CheckBox x:Name=""SoundCheck"" Content=""Play a sound when the time is up"" IsChecked=""True"" FontSize=""17"" Foreground=""#4A4D57"" HorizontalAlignment=""Center"" Margin=""0,44,0,0"" VerticalContentAlignment=""Center""/>
    </StackPanel>
    <Border Grid.Column=""1"" Background=""White"" BorderBrush=""#E6E7EB"" BorderThickness=""1"" CornerRadius=""18"" Padding=""30,26"" VerticalAlignment=""Center"">
      <StackPanel>
        <TextBlock x:Name=""TodayHeading"" Text=""Today"" FontSize=""24"" FontWeight=""SemiBold"" Foreground=""#17181C""/>
        <TextBlock x:Name=""TodayTotal"" Text=""3 sessions, 1 h 15 min"" FontSize=""16"" Foreground=""#8A8D97"" Margin=""0,2,0,18""/>
        <ListBox x:Name=""SessionList"" BorderThickness=""0"" FontSize=""18"" Foreground=""#2A2C33"" Background=""Transparent"">
          <ListBoxItem Padding=""2,10"">09:10   Write the invoice export</ListBoxItem>
          <ListBoxItem Padding=""2,10"">10:05   Review pull requests</ListBoxItem>
          <ListBoxItem Padding=""2,10"">11:30   Fix the search filter</ListBoxItem>
        </ListBox>
        <Button x:Name=""ClearButton"" Content=""Clear today"" HorizontalAlignment=""Left"" Margin=""0,20,0,0"" FontSize=""16"" Padding=""20,10""/>
      </StackPanel>
    </Border>
  </Grid>
</Window>";
  [STAThread] static void Main() {
    var w = (Window)XamlReader.Parse(Xaml);
    new Application().Run(w);
  }
}
