using System.Reflection;
using Spectre.Console;
using Spectre.Console.Cli;

namespace Wolfware.Moonlit.Cli.Commands;

public sealed class VersionCommand : Command
{
  public const string Name = "version";
  public const string Description = "Show the version of the Moonlit CLI";

  public override int Execute(CommandContext context)
  {
    var header = new FigletText("Moonlit CLI").LeftJustified().Color(Color.DarkGoldenrod);
    var paddedHeader = new Padder(header).PadBottom(2).PadTop(2).PadLeft(0).PadRight(0);
    AnsiConsole.Write(paddedHeader);

    var assembly = this.GetType().Assembly;
    var informationalVersion = assembly.GetCustomAttribute<AssemblyInformationalVersionAttribute>();
    AnsiConsole.MarkupLineInterpolated(
      $"[bold steelblue]Version:[/] {informationalVersion?.InformationalVersion.Split('+')[0] ?? "Unknown"}");
    AnsiConsole.MarkupLine("[bold steelblue]Author:[/] Wolfware LLC");
    AnsiConsole.MarkupLine("[bold steelblue]License:[/] MIT License");

    return 0;
  }
}
