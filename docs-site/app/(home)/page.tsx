import Link from 'next/link';

const cards = [
  {
    href: '/docs/getting-started',
    title: 'Getting started',
    body: 'Run the whole stack locally: prerequisites, one Docker Compose command, every port, seed data, troubleshooting.',
  },
  {
    href: '/docs/architecture',
    title: 'Architecture',
    body: 'The stack, the repository layout, the backend and frontend, the data model, events/Kafka, offline behaviour, security.',
  },
  {
    href: '/docs/reference',
    title: 'API reference',
    body: 'Every endpoint, generated from the backend’s own OpenAPI document — base URLs, auth, errors, pagination, one page per resource.',
  },
  {
    href: '/docs/operations',
    title: 'Environments & operations',
    body: 'Demo/dev/staging/prod, monitoring, Kubernetes, backups, platform accounts, the pilot launch checklist.',
  },
  {
    href: '/docs/contracts-and-design',
    title: 'Contracts & design',
    body: 'The reviewed design behind the API and data model, including the proposed, not-yet-built payments design.',
  },
  {
    href: '/docs/status',
    title: 'Status & progress',
    body: 'What exists today with evidence, the full engineering log, the acceptance matrix, and open decisions.',
  },
];

export default function HomePage() {
  return (
    <main className="mx-auto flex w-full max-w-4xl flex-1 flex-col justify-center gap-8 px-4 py-16">
      <div className="flex flex-col gap-3">
        <h1 className="text-3xl font-bold sm:text-4xl">StockLink documentation</h1>
        <p className="text-fd-muted-foreground text-lg">
          Architecture, the API reference, environments, contracts and current status — generated
          from an explicit manifest of the markdown under <code>docs/</code>, so the site and the
          repo never drift.
        </p>
      </div>
      <div className="grid gap-4 sm:grid-cols-2">
        {cards.map((card) => (
          <Link
            key={card.href}
            href={card.href}
            className="rounded-xl border border-fd-border bg-fd-card p-5 transition-colors hover:bg-fd-accent"
          >
            <div className="font-semibold">{card.title}</div>
            <p className="text-fd-muted-foreground mt-1 text-sm">{card.body}</p>
          </Link>
        ))}
      </div>
      <Link href="/docs" className="text-fd-muted-foreground text-sm underline underline-offset-4">
        Or start at the top: Start here
      </Link>
    </main>
  );
}
