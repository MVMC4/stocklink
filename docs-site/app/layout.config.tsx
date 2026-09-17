import type { BaseLayoutProps } from 'fumadocs-ui/layouts/shared';

/**
 * Shared layout options (nav title, links) used by both the home and docs
 * layouts.
 */
export const baseOptions: BaseLayoutProps = {
  nav: {
    title: (
      <span className="font-semibold">
        📦 StockLink <span className="text-fd-muted-foreground">docs</span>
      </span>
    ),
  },
  links: [
    {
      text: 'Documentation',
      url: '/docs',
      active: 'nested-url',
    },
  ],
  githubUrl: 'https://github.com/MVMC4/stocklink',
};
