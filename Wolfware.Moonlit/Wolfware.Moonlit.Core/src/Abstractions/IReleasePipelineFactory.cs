using Wolfware.Moonlit.Core.Configuration;
using Wolfware.Moonlit.Core.Pipelines;

namespace Wolfware.Moonlit.Core.Abstractions;

public interface IReleasePipelineFactory
{
  Task<ReleasePipeline> Create(ReleaseConfiguration configuration);
}
