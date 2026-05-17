"use client";

import { motion, useScroll, useSpring } from "framer-motion";

/**
 * Top scroll-progress rail. Reads native window scroll (the SectionSnap
 * tween writes it every frame), spring-smoothed so it glides between
 * page jumps rather than stepping. Pure wayfinding — never intercepts
 * scroll. This is the thin top accent line, NOT a side scrollbar.
 */
export function ScrollProgress() {
  const { scrollYProgress } = useScroll();
  const scaleX = useSpring(scrollYProgress, {
    stiffness: 120,
    damping: 30,
    restDelta: 0.001,
  });

  return <motion.div className="scroll-rail" style={{ scaleX }} />;
}
