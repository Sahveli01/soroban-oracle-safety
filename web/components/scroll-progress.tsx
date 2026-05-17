"use client";

import { motion, useScroll, useSpring } from "framer-motion";

/**
 * Top scroll-progress rail. Reads native scroll (Lenis updates it),
 * spring-smoothed so it glides rather than tracks 1:1. Pure wayfinding
 * for a long single-page document — never intercepts scroll.
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
