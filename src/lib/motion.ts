import type { Variants, Transition } from 'framer-motion';

/* ─── Shared transitions ─── */

export const springSnappy: Transition = {
  type: 'spring',
  stiffness: 400,
  damping: 25,
  mass: 0.8,
};

export const springGentle: Transition = {
  type: 'spring',
  stiffness: 200,
  damping: 20,
  mass: 1,
};

export const springBouncy: Transition = {
  type: 'spring',
  stiffness: 500,
  damping: 20,
  mass: 0.6,
};

/* ─── Tab transitions (slide + fade) ─── */

export const tabSlide: Variants = {
  enter: (direction: number) => ({
    x: direction > 0 ? 20 : -20,
    opacity: 0,
  }),
  center: {
    x: 0,
    opacity: 1,
    transition: springGentle,
  },
  exit: (direction: number) => ({
    x: direction > 0 ? -20 : 20,
    opacity: 0,
    transition: { duration: 0.15 },
  }),
};

/* ─── Fade in (general purpose) ─── */

export const fadeIn: Variants = {
  hidden: { opacity: 0 },
  visible: {
    opacity: 1,
    transition: { duration: 0.2 },
  },
};
