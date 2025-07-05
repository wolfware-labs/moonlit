using LibGit2Sharp;
using Microsoft.Extensions.Configuration;
using Wolfware.Moonlit.Plugins.Abstractions;
using Wolfware.Moonlit.Plugins.Git.Extensions;
using Wolfware.Moonlit.Plugins.Pipeline;

namespace Wolfware.Moonlit.Plugins.Git.Middlewares;

public sealed class GetRepositoryContext : IReleaseMiddleware
{
  public Task<MiddlewareResult> ExecuteAsync(ReleaseContext context, IConfiguration configuration)
  {
    var gitFolderPath = context.WorkingDirectory.GetGitFolderPath();
    using var repo = new Repository(gitFolderPath);
    var currentBranch = repo.Head.FriendlyName;
    var remoteUrl = repo.Network.Remotes["origin"].Url;

    return Task.FromResult(MiddlewareResult.Success(output =>
    {
      output.Add("Branch", currentBranch);
      output.Add("RemoteUrl", remoteUrl);
    }));
  }
}
