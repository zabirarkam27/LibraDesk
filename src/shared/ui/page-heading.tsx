export function PageHeading({ label, title, description }: { label: string; title: string; description: string }) {
  return (
    <header className="mb-7 space-y-2">
      <p className="eyebrow">{label}</p>
      <h1 className="font-display text-4xl tracking-tight sm:text-5xl">{title}</h1>
      <p className="max-w-2xl text-sm leading-6 text-muted-foreground">{description}</p>
    </header>
  );
}
