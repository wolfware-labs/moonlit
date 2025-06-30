using Spectre.Console;
using Spectre.Console.Cli;
using Wolfware.Moonlit.Cli.Logging;
using Wolfware.Moonlit.Core.Abstractions;
using Wolfware.Moonlit.Core.Pipelines;
using Wolfware.Moonlit.Core.Plugins;
using Wolfware.Moonlit.Plugins.Pipeline;

namespace Wolfware.Moonlit.Cli.Commands;

public sealed class ReleaseCommand : AsyncCommand<ReleaseCommand.Settings>
{
  private readonly IReleaseConfigurationParser _configurationParser;
  private readonly IReleasePipelineFactory _releasePipelineFactory;

  public const string Name = "release";
  public const string Description = "Executes a release pipeline";

  public sealed class Settings : CommandSettings
  {
    [CommandOption("-w|--working-directory <workingDirectory>")]
    public string WorkingDirectory { get; set; } = Environment.CurrentDirectory;

    [CommandOption("-f|--file <fileName>")]
    public string FileName { get; set; } = "release.yml";

    [CommandOption("-s|--stages <stages>")]
    public string[] Stages { get; set; } = [];

    [CommandOption("-v|--verbose")]
    public bool Verbose { get; set; } = false;

    [CommandOption("-a|--arg <argument>")]
    public string[] Arguments { get; set; } = [];

    public string ConfigurationFilePath => Path.Combine(WorkingDirectory, FileName);

    public override ValidationResult Validate()
    {
      if (string.IsNullOrEmpty(WorkingDirectory) || !Directory.Exists(WorkingDirectory))
      {
        return ValidationResult.Error("The specified working directory does not exist.");
      }

      if (string.IsNullOrEmpty(FileName))
      {
        return ValidationResult.Error("The configuration file name cannot be empty.");
      }

      if (!FileName.EndsWith(".yml", StringComparison.OrdinalIgnoreCase) &&
          !FileName.EndsWith(".yaml", StringComparison.OrdinalIgnoreCase))
      {
        return ValidationResult.Error("The configuration file must be a YAML file (ending with .yml or .yaml).");
      }

      if (!File.Exists(this.ConfigurationFilePath))
      {
        return ValidationResult.Error(
          $"The configuration file '{FileName}' does not exist in the specified working directory.");
      }

      return ValidationResult.Success();
    }
  }

  public ReleaseCommand(IReleaseConfigurationParser configurationParser, IReleasePipelineFactory releasePipelineFactory)
  {
    _configurationParser = configurationParser;
    _releasePipelineFactory = releasePipelineFactory;
  }

  public override async Task<int> ExecuteAsync(CommandContext context, Settings settings)
  {
    try
    {
      var configurationContent = await File.ReadAllTextAsync(settings.ConfigurationFilePath).ConfigureAwait(false);
      var configuration = await _configurationParser.Parse(configurationContent).ConfigureAwait(false);

      if (configuration.Stages.Count == 0)
      {
        AnsiConsole.MarkupLine("[red]No stages found in the release configuration.[/]");
        return 1;
      }

      if (settings.Stages.Length > 0)
      {
        var configuredStages = configuration.Stages.Keys.ToArray();
        foreach (var configuredStage in configuredStages)
        {
          if (!settings.Stages.Contains(configuredStage, StringComparer.OrdinalIgnoreCase))
          {
            configuration.Stages.Remove(configuredStage);
          }
        }

        if (configuration.Stages.Count == 0)
        {
          AnsiConsole.MarkupLine("[red]No matching stages found in the release configuration.[/]");
          return 1;
        }
      }

      if (settings.Arguments.Length > 0)
      {
        foreach (var argument in settings.Arguments)
        {
          var parts = argument.Split('=', 2);
          if (parts.Length == 2)
          {
            configuration.Arguments[parts[0].Trim()] = parts[1].Trim();
          }
          else
          {
            AnsiConsole.MarkupLineInterpolated($"[red]Invalid argument format: {argument}[/]");
            return 1;
          }
        }
      }


      AnsiConsole.MarkupLineInterpolated($":rocket: [green]Executing release pipeline:[/] {configuration.Name}");
      AnsiConsole.MarkupLineInterpolated($":file_folder: [blue]Working Directory:[/] {settings.WorkingDirectory}");
      AnsiConsole.MarkupLineInterpolated($":gear: [blue]Configuration File:[/] {settings.FileName}");

      if (settings.Verbose)
      {
        AnsiConsole.MarkupLineInterpolated($"[blue]Configuration Content:[/] {configuration}");
      }

      AnsiConsole.WriteLine();

      await using var pipeline = await this._releasePipelineFactory.Create(configuration).ConfigureAwait(false);

      var response = await AnsiConsole.Status()
        .Spinner(Spinner.Known.Moon)
        .SpinnerStyle(Style.Parse("khaki3 bold"))
        .StartAsync(
          "Executing Release...",
          _ => pipeline.ExecuteAsync(new PipelineContext
          {
            WorkingDirectory = settings.WorkingDirectory,
            Logger = new ConsoleLogger(settings.Verbose),
            CancellationToken = CancellationToken.None // TODO: Handle cancellation token properly
          })
        ).ConfigureAwait(false);

      AnsiConsole.WriteLine();

      if (response.IsSuccessful)
      {
        AnsiConsole.MarkupLine(":check_mark_button: [green]Release completed[/]");
        return 0;
      }

      AnsiConsole.MarkupLineInterpolated($"\n[red]Release failed:[/] {response.ErrorMessage}");
      return 1;
    }
    catch (Exception e)
    {
      if (settings.Verbose)
      {
        AnsiConsole.WriteException(e, ExceptionFormats.ShortenEverything);
      }
      else
      {
        AnsiConsole.MarkupLineInterpolated($"[red]An error occurred while executing release command: [/] {e.Message}");
      }

      return 1;
    }
  }
}
