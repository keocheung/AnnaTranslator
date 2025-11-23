let openAIConstructor: typeof import("openai").default | null = null;

export async function getOpenAIConstructor() {
  if (openAIConstructor) return openAIConstructor;
  const mod = await import("openai");
  openAIConstructor = mod.default;
  return openAIConstructor;
}
