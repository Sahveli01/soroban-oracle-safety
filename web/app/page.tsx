import { Nav } from "@/components/nav";
import { Hero } from "@/components/hero";
import { Attack } from "@/components/sections/attack";
import { Solution } from "@/components/sections/solution";
import { HowItWorks } from "@/components/sections/how-it-works";
import { Architecture } from "@/components/sections/architecture";
import { Mechanism } from "@/components/sections/mechanism";
import { Infrastructure } from "@/components/sections/infrastructure";
import { Operator } from "@/components/sections/operator";
import { Live } from "@/components/sections/live";
import { Audit } from "@/components/sections/audit";
import { Footer } from "@/components/sections/footer";

export default function Home() {
  return (
    <main className="min-h-screen">
      <Nav />
      {/* Stack context: consecutive sticky panels slide over each other
          (the page-turn). Must stay transform/overflow free so `position:
          sticky` resolves against the viewport. */}
      <div className="relative">
        <Hero />
        <Attack />
        <Solution />
        <HowItWorks />
        <Architecture />
        <Mechanism />
        <Infrastructure />
        <Operator />
        <Live />
        <Audit />
        {/* Footer is the final sticky panel of the SAME stack — it
            page-turns in like every section and closes seamlessly,
            so the end is one continuous page, not two. */}
        <Footer />
      </div>
    </main>
  );
}
