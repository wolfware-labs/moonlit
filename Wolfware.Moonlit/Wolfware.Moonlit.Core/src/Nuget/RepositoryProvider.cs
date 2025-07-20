using NuGet.Configuration;
using NuGet.Protocol;
using NuGet.Protocol.Core.Types;
using Wolfware.Moonlit.Core.Nuget.Abstractions;

namespace Wolfware.Moonlit.Core.Nuget;

public sealed class RepositoryProvider : IRepositoryProvider
{
  private readonly Dictionary<string, SourceRepository> _repositories = new();

  public RepositoryProvider()
  {
    var settings = Settings.LoadDefaultSettings(Environment.CurrentDirectory);
    var packageSourceProvider = new PackageSourceProvider(settings);
    packageSourceProvider.LoadPackageSources()
      .Where(x => x.IsEnabled)
      .ToList()
      .ForEach(x =>
        _repositories.TryAdd(RepositoryProvider.NormalizeRepositoryName(x.Name), Repository.Factory.GetCoreV3(x))
      );
  }

  public SourceRepository GetRepository(string repositoryId)
  {
    ArgumentNullException.ThrowIfNull(repositoryId);
    return _repositories.TryGetValue(repositoryId, out var repository)
      ? repository
      : throw new NotSupportedException($"Repository '{repositoryId}' not found");
  }

  private static string NormalizeRepositoryName(string repositoryName)
  {
    return repositoryName.Equals("Microsoft Visual Studio Offline Packages")
      ? "offline"
      : repositoryName.ToLowerInvariant().Replace(" ", "-");
  }
}
