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

/* ─── Popover mount / unmount ─── */

export const popoverMount: Variants = {
  hidden: {
    opacity: 0,
    scale: 0.96,
    y: -4,
  },
  visible: {
    opacity: 1,
    scale: 1,
    y: 0,
    transition: springSnappy,
  },
  exit: {
    opacity: 0,
    scale: 0.96,
    y: -4,
    transition: { duration: 0.15 },
  },
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

/* ─── Number ticker (count up on update) ─── */

export const numberTick: Variants = {
  initial: { opacity: 0.7, y: 4 },
  animate: {
    opacity: 1,
    y: 0,
    transition: springSnappy,
  },
};

/* ─── Progress bar fill (spring, not linear) ─── */

export const barFill: Variants = {
  initial: { scaleX: 0, originX: 0 },
  animate: {
    scaleX: 1,
    transition: {
      ...springSnappy,
      duration: 0.8,
    },
  },
};

/* ─── Stale data pulse ─── */

export const stalePulse: Variants = {
  active: {
    opacity: [1, 0.5, 1],
    transition: {
      duration: 2,
      repeat: Infinity,
      ease: 'easeInOut',
    },
  },
  inactive: {
    opacity: 1,
  },
};

/* ─── Threshold crossed flash ─── */

export const thresholdFlash: Variants = {
  initial: { boxShadow: '0 0 0 0 rgba(251, 146, 60, 0)' },
  flash: {
    boxShadow: [
      '0 0 0 0 rgba(251, 146, 60, 0.4)',
      '0 0 12px 4px rgba(251, 146, 60, 0.2)',
      '0 0 0 0 rgba(251, 146, 60, 0)',
    ],
    transition: { duration: 0.6 },
  },
};

/* ─── Card stagger entrance ─── */

export const cardStagger: Variants = {
  hidden: {},
  visible: {
    transition: {
      staggerChildren: 0.05,
    },
  },
};

export const cardChild: Variants = {
  hidden: { opacity: 0, y: 8 },
  visible: {
    opacity: 1,
    y: 0,
    transition: springGentle,
  },
};

/* ─── Fade in (general purpose) ─── */

export const fadeIn: Variants = {
  hidden: { opacity: 0 },
  visible: {
    opacity: 1,
    transition: { duration: 0.2 },
  },
};
