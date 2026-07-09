import { parseCodexDoctorReport, parseCodexModelCatalog } from './codexRuntimeInfoProvider';

describe('parseCodexModelCatalog', () => {
  it('normalizes bundled Codex model catalog output', () => {
    const result = parseCodexModelCatalog(
      JSON.stringify({
        models: [
          {
            slug: 'gpt-5.5',
            display_name: 'GPT-5.5',
            priority: 0,
            default_reasoning_level: 'medium',
            supported_reasoning_levels: [
              { effort: 'low', description: 'Fast responses with lighter reasoning' },
              { effort: 'medium', description: 'Balanced' },
              { effort: 'high', description: 'Greater depth' },
              { effort: 'xhigh', description: 'Extra high depth' },
            ],
          },
          {
            slug: 'gpt-5.3-codex-spark',
            reasoningEfforts: ['minimal', 'low'],
          },
        ],
      }),
    );

    expect(result).toEqual({
      models: [
        {
          id: 'gpt-5.5',
          name: 'GPT-5.5',
          recommended: true,
          defaultReasoningEffort: 'medium',
          reasoningEfforts: ['low', 'medium', 'high', 'xhigh'],
        },
        {
          id: 'gpt-5.3-codex-spark',
          reasoningEfforts: ['minimal', 'low'],
        },
      ],
      recommendedModel: 'gpt-5.5',
      reasoningEfforts: ['low', 'medium', 'high', 'xhigh', 'minimal'],
      source: 'codex-debug-models',
    });
  });
});

describe('parseCodexDoctorReport', () => {
  it('extracts configured runtime defaults from doctor json', () => {
    const result = parseCodexDoctorReport(
      JSON.stringify({
        overallStatus: 'warning',
        codexVersion: '0.143.0',
        checks: {
          'config.load': {
            details: {
              'config.toml': 'C:\\Users\\user\\.codex\\config.toml',
              model: 'gpt-5.5',
              'model provider': 'openai',
            },
          },
          'runtime.provenance': {
            details: {
              version: '0.143.0',
            },
          },
        },
      }),
    );

    expect(result).toEqual({
      configuredModel: 'gpt-5.5',
      modelProvider: 'openai',
      codexVersion: '0.143.0',
      configPath: 'C:\\Users\\user\\.codex\\config.toml',
      healthStatus: 'warning',
    });
  });
});
