import { useEffect, useState } from 'react';

const MOBILE_BREAKPOINT = 768;

export function useIsDesktop() {
  // Start from the real width. Reporting mobile on the first render makes
  // desktop-only panels mount a step late, after layout has settled without
  // them.
  const [isDesktop, setIsDesktop] = useState(
    () => window.innerWidth > MOBILE_BREAKPOINT,
  );

  useEffect(() => {
    const mql = window.matchMedia(`(max-width: ${MOBILE_BREAKPOINT - 1}px)`);
    const onChange = () => {
      setIsDesktop(window.innerWidth > MOBILE_BREAKPOINT);
    };

    mql.addEventListener('change', onChange);
    setIsDesktop(window.innerWidth > MOBILE_BREAKPOINT);

    return () => {
      mql.removeEventListener('change', onChange);
    };
  }, []);

  return isDesktop;
}
