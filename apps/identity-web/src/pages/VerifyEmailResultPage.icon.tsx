import { useEffect, useRef, type ReactNode } from 'react';

export function AnimatedResultIcon({ children }: { children: ReactNode }) {
  const ref = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const element = ref.current;
    if (!element) {
      return;
    }

    element.classList.remove('nvbes-verify-icon-enter');
    void element.offsetWidth;
    element.classList.add('nvbes-verify-icon-enter');
  }, []);

  return (
    <>
      <style>{`
        @keyframes nvbes-verify-icon-pop {
          0% { transform: scale(0); opacity: 0; }
          60% { transform: scale(1.15); }
          100% { transform: scale(1); opacity: 1; }
        }
        .nvbes-verify-icon-enter {
          animation: nvbes-verify-icon-pop 0.4s cubic-bezier(0.16, 1, 0.3, 1) both;
        }
      `}</style>
      <div ref={ref} className="nvbes-verify-icon-enter">
        {children}
      </div>
    </>
  );
}
