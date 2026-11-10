import { ArrowRight, BookOpen, Settings2 } from "lucide-react";
import { LazyMotion, useReducedMotion } from "framer-motion";
import { section as MotionSection } from "framer-motion/m";
import { Link } from "react-router";
import { Button } from "@/shared/ui/button";
import { PageHeading } from "@/shared/ui/page-heading";

const loadAnimationFeatures = () => import("./animation-features").then(({ default: features }) => features);

export function OverviewPage() {
  const reducedMotion = useReducedMotion();

  return (
    <>
      <PageHeading label="Overview" title="Your reading room" description="A local home for your PDF library." />
      <LazyMotion features={loadAnimationFeatures} strict>
        <MotionSection
          initial={reducedMotion ? false : { y: 8 }}
          animate={{ y: 0 }}
          transition={{ duration: 0.22 }}
          className="relative isolate overflow-hidden rounded-xl border bg-card p-6 sm:p-9"
          aria-labelledby="workspace-heading"
        >
          <div className="book-art hidden lg:block" aria-hidden="true">
            <span className="book book-one" />
            <span className="book book-two" />
            <span className="book book-three" />
          </div>
          <div className="relative max-w-md">
            <p className="eyebrow mb-4">Library workspace</p>
            <h2 id="workspace-heading" className="font-display text-3xl tracking-tight">Make room for good books.</h2>
            <p className="mt-3 max-w-sm text-sm leading-6 text-muted-foreground">Open your library workspace, visit the reader, or set the appearance for each.</p>
            <div className="mt-5 flex flex-wrap gap-2">
              <Button asChild><Link to="/import">Import PDFs <ArrowRight /></Link></Button>
              <Button asChild variant="outline"><Link to="/library">Open library</Link></Button>
            </div>
          </div>
        </MotionSection>
      </LazyMotion>
      <div className="mt-5 grid gap-4 sm:grid-cols-2">
        <WorkspaceLink to="/reader" title="Reader workspace" description="Preview your reading appearance." icon={BookOpen} />
        <WorkspaceLink to="/settings" title="Preferences" description="Manage appearance and check local storage." icon={Settings2} />
      </div>
    </>
  );
}
function WorkspaceLink({ to, title, description, icon: Icon }: { to: string; title: string; description: string; icon: typeof BookOpen }) {
  return (
    <Link to={to} className="group flex items-start gap-3 rounded-lg border bg-card p-4 hover:border-ring focus-visible:outline-2 focus-visible:outline-ring">
      <span className="grid size-9 shrink-0 place-items-center rounded-md bg-accent text-accent-foreground"><Icon className="size-4" /></span>
      <span className="min-w-0 flex-1">
        <span className="block text-sm font-semibold">{title}</span>
        <span className="mt-1 block text-sm leading-5 text-muted-foreground">{description}</span>
      </span>
      <ArrowRight className="mt-2 size-4 shrink-0 text-muted-foreground transition-colors group-hover:text-foreground" />
    </Link>
  );
}
