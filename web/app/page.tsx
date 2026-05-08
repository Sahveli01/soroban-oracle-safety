import { Nav } from "@/components/nav";
import { Hero } from "@/components/hero";

export default function Home() {
  return (
    <main className="min-h-screen">
      <Nav />
      <Hero />
      {/* Phase 8.2 will add: Attack, Solution, How It Works, Architecture, Live, Audit, Footer */}
    </main>
  );
}
