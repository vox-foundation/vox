import type React from 'react';

declare global {
  namespace JSX {
    interface IntrinsicElements {
      column: React.HTMLAttributes<HTMLDivElement>;
      row: React.HTMLAttributes<HTMLDivElement>;
      panel: React.HTMLAttributes<HTMLDivElement>;
      text: React.HTMLAttributes<HTMLSpanElement>;
      heading: React.HTMLAttributes<HTMLHeadingElement> & { level?: string | number };
      badge: React.HTMLAttributes<HTMLSpanElement>;
      icon: React.HTMLAttributes<HTMLSpanElement> & { name?: string };
      spacer: React.HTMLAttributes<HTMLDivElement>;
      divider: React.HTMLAttributes<HTMLHRElement>;
      scrollview: React.HTMLAttributes<HTMLDivElement>;
      stack: React.HTMLAttributes<HTMLDivElement>;
    }
  }
}

export {};
