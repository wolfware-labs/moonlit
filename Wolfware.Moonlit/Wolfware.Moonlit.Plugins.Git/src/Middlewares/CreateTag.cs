using LibGit2Sharp;
using Microsoft.Extensions.Configuration;
using Microsoft.Extensions.Logging;
using Microsoft.Extensions.Options;
using Wolfware.Moonlit.Plugins.Abstractions;
using Wolfware.Moonlit.Plugins.Extensions;
using Wolfware.Moonlit.Plugins.Git.Configuration;
using Wolfware.Moonlit.Plugins.Git.Extensions;
using Wolfware.Moonlit.Plugins.Pipeline;

namespace Wolfware.Moonlit.Plugins.Git.Middlewares;

public sealed class CreateTag : ReleaseMiddleware<CreateTag.Configuration>
{
  private GitConfiguration _gitConfiguration;

  public sealed class Configuration
  {
    public string Format { get; set; } = "{0}";

    public string Value { get; set; } = string.Empty;
  }

  public CreateTag(IOptions<GitConfiguration> gitOptions)
  {
    this._gitConfiguration = gitOptions.Value;
  }

  public override Task<MiddlewareResult> ExecuteAsync(PipelineContext context, Configuration configuration)
  {
    var gitFolderPath = context.WorkingDirectory.GetGitFolderPath();
    using var gitRepo = new Repository(gitFolderPath);
    var tagName = string.Format(configuration.Format, configuration.Value);
    if (string.IsNullOrWhiteSpace(tagName))
    {
      return Task.FromResult(MiddlewareResult.Failure("Tag name cannot be null or empty."));
    }

    if (gitRepo.Tags[tagName] != null)
    {
      return Task.FromResult(MiddlewareResult.Failure($"Tag '{tagName}' already exists in the repository."));
    }

    var commit = gitRepo.Head.Tip;
    if (commit == null)
    {
      return Task.FromResult(MiddlewareResult.Failure("No commits found in the repository to tag."));
    }

    var tag = gitRepo.Tags.Add(tagName, commit);

    var pushOptions = new PushOptions
    {
      CredentialsProvider = (_, _, _) => new UsernamePasswordCredentials
      {
        Username = this._gitConfiguration.Username, Password = this._gitConfiguration.Password
      }
    };
    var remote = gitRepo.Network.Remotes["origin"];
    gitRepo.Network.Push(remote, $"refs/tags/{tag.FriendlyName}", pushOptions);
    context.Logger.LogInformation("Created tag '{TagName}' at commit {CommitId}.", tag.FriendlyName, commit.Sha);
    return Task.FromResult(MiddlewareResult.Success(output =>
    {
      output.Add("TagName", tag.FriendlyName);
      output.Add("CommitId", commit.Sha);
    }));
  }
}
