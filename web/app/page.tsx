import { Deck, type DeckSlide } from "@/components/deck";
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
import { Trust } from "@/components/sections/trust";
import { Footer } from "@/components/sections/footer";

// Ordered slide deck. Each is one full-viewport page; the deck moves
// between them by integer index (no scroll) — see components/deck.tsx.
const SLIDES: DeckSlide[] = [
  { id: "hero", node: <Hero /> },
  { id: "attack", node: <Attack /> },
  { id: "solution", node: <Solution /> },
  { id: "how-it-works", node: <HowItWorks /> },
  { id: "architecture", node: <Architecture /> },
  { id: "mechanism", node: <Mechanism /> },
  { id: "infrastructure", node: <Infrastructure /> },
  { id: "operator", node: <Operator /> },
  { id: "live", node: <Live /> },
  { id: "trust", node: <Trust /> },
  { id: "footer", node: <Footer /> },
];

export default function Home() {
  return (
    <>
      <Nav />
      <main>
        <Deck slides={SLIDES} />
      </main>
    </>
  );
}
