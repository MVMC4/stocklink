import { createMDX } from 'fumadocs-mdx/next';

const withMDX = createMDX();

/** @type {import('next').NextConfig} */
const config = {
  reactStrictMode: true,
  // Standalone tracing creates symlinks that standard Windows developer
  // accounts cannot always create. Docker opts in for the deployable bundle;
  // local verification still exercises the full production compilation.
  ...(process.env.NEXT_STANDALONE === 'true' ? { output: 'standalone' } : {}),
  pageExtensions: ['ts', 'tsx', 'js', 'jsx', 'md', 'mdx'],
};

export default withMDX(config);
