using System.Runtime.InteropServices;
using System.Runtime.Versioning;
using NuGet.Frameworks;
using NuGet.Packaging;

namespace Wolfware.Moonlit.Core.Nuget;

public static class NugetRuntime
{
  public static NuGetFramework CurrentNugetFramework { get; } = NugetRuntime.GetCurrentFramework();

  public static string CurrentRuntimeIdentifier { get; } = NugetRuntime.GetCurrentRuntimeIdentifier();

  public static bool IsRuntimeCompatible(string runtimeIdentifier)
  {
    return runtimeIdentifier.Contains(NugetRuntime.CurrentRuntimeIdentifier) ||
           runtimeIdentifier.Equals("any", StringComparison.OrdinalIgnoreCase);
  }

  public static TGroup? GetMostCompatibleGroup<TGroup>(TGroup[] groups) where TGroup : class, IFrameworkSpecific
  {
    var reducer = new FrameworkReducer();
    var frameworks = groups.Select(g => g.TargetFramework).ToList();
    var nearest = reducer.GetNearest(NugetRuntime.CurrentNugetFramework, frameworks);
    return nearest != null ? groups.FirstOrDefault(g => g.TargetFramework.Equals(nearest)) : null;
  }

  private static NuGetFramework GetCurrentFramework()
  {
    var frameworkName = new FrameworkName(
      AppDomain.CurrentDomain.SetupInformation.TargetFrameworkName ?? ".NETCoreApp,Version=v9.0"
    );

    return NuGetFramework.ParseFrameworkName(frameworkName.ToString(), DefaultFrameworkNameProvider.Instance);
  }

  private static string GetCurrentRuntimeIdentifier()
  {
    var architecture = RuntimeInformation.OSArchitecture.ToString().ToLower();
    if (RuntimeInformation.IsOSPlatform(OSPlatform.Windows))
      return $"win-{architecture}";
    if (RuntimeInformation.IsOSPlatform(OSPlatform.Linux))
      return $"linux-{architecture}";
    return RuntimeInformation.IsOSPlatform(OSPlatform.OSX) ? $"osx-{architecture}" : "any";
  }
}
