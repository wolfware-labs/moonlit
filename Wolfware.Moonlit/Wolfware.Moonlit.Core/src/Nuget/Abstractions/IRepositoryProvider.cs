using NuGet.Protocol.Core.Types;

namespace Wolfware.Moonlit.Core.Nuget.Abstractions;

public interface IRepositoryProvider
{
  public SourceRepository? GetRepository(string repositoryId);
}
