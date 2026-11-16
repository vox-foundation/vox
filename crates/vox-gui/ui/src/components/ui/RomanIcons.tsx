import React from 'react';

export const RomanIcon = {
  romanColumn: (p: React.SVGProps<SVGSVGElement>) => (
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" {...p}>
      <path d="M4 5h16M5 8h14M4 19h16M5 21h14" />
      <path d="M6 5a2 2 0 1 0 0 3M18 5a2 2 0 1 1 0 3" />
      <path d="M8 8v11M12 8v11M16 8v11" />
    </svg>
  ),
  laurelWreath: (p: React.SVGProps<SVGSVGElement>) => (
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" {...p}>
      <path d="M12 20c-4.5-1.5-7-5.5-7-10 0-2.5 1-5 3-6.5" />
      <path d="M12 20c4.5-1.5 7-5.5 7-10 0-2.5-1-5-3-6.5" />
      <path d="M9 21l3-2 3 2" />
      <path d="M5.5 12l2.5-1M5.5 8.5l2.5-.5M7 15.5l2.5-1.5" />
      <path d="M18.5 12l-2.5-1M18.5 8.5l-2.5-.5M17 15.5l-2.5-1.5" />
    </svg>
  ),
  gladiusShield: (p: React.SVGProps<SVGSVGElement>) => (
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" {...p}>
      <path d="M6 3v13l1.5 2L9 16V3L7.5 1.5 6 3Z" />
      <path d="M4.5 14h6M7.5 18v3" />
      <rect x="12" y="4" width="8" height="14" rx="2" />
      <circle cx="16" cy="11" r="1.8" />
      <path d="M16 4v5M16 13v5M12 11h2.2M17.8 11H20" />
    </svg>
  ),
  denariusCoin: (p: React.SVGProps<SVGSVGElement>) => (
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" {...p}>
      <circle cx="12" cy="12" r="9" />
      <circle cx="12" cy="12" r="6.5" strokeDasharray="2 2" />
      <path d="M10 8.5h3a2 2 0 0 1 0 4h-2a2 2 0 0 0 0 4h3M12 7v10" />
    </svg>
  ),
  hourglassVia: (p: React.SVGProps<SVGSVGElement>) => (
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" {...p}>
      <path d="M5 3h14M5 21h14" />
      <path d="M7 3v4c0 2.5 5 3.5 5 5s-5 2.5-5 5v4" />
      <path d="M17 3v4c0 2.5-5 3.5-5 5s5 2.5 5 5v4" />
    </svg>
  ),
  brazierTorch: (p: React.SVGProps<SVGSVGElement>) => (
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" {...p}>
      <path d="M12 2c1.5 2.5 3.5 4 3.5 6.8a3.5 3.5 0 1 1-7 0C8.5 6 10.5 4.5 12 2Z" />
      <path d="M7 12h10l-1.5 3.5h-7L7 12Z" />
      <path d="M10.5 15.5L9 21M13.5 15.5L15 21M12 15.5V21" />
    </svg>
  ),
  astrolabeRadar: (p: React.SVGProps<SVGSVGElement>) => (
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" {...p}>
      <circle cx="12" cy="12" r="9" />
      <circle cx="12" cy="12" r="5" />
      <circle cx="12" cy="12" r="1.5" />
      <path d="M12 3v3M12 18v3M3 12h3M18 12h3" />
      <path d="M12 12l4.5-4.5" />
    </svg>
  ),
  aqueductArch: (p: React.SVGProps<SVGSVGElement>) => (
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" {...p}>
      <path d="M2 6h20M2 9h20" />
      <path d="M3 20V9M9 20v-5a3 3 0 0 0-6 0v5" />
      <path d="M15 20v-5a3 3 0 0 0-6 0v5" />
      <path d="M21 20v-5a3 3 0 0 0-6 0v5" />
    </svg>
  ),
  waxSeal: (p: React.SVGProps<SVGSVGElement>) => (
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" {...p}>
      <path d="M12 2.5l2.4 1.3 2.7-.3 1.3 2.4 2.4 1.3-.3 2.7 1.3 2.4-1.3 2.4.3 2.7-2.4 1.3-1.3 2.4-2.7-.3-2.4 1.3-2.4-1.3-2.7.3-1.3-2.4-2.4-1.3.3-2.7L2.2 12l1.3-2.4-.3-2.7 2.4-1.3 1.3-2.4 2.7.3L12 2.5Z" />
      <circle cx="12" cy="12" r="5" />
      <path d="M9.5 12l1.8 1.8 3.4-3.6" />
    </svg>
  ),
  scrollTabula: (p: React.SVGProps<SVGSVGElement>) => (
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" {...p}>
      <path d="M6 4h11a2 2 0 0 1 2 2v12a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2V6a2 2 0 0 1 2-2Z" />
      <path d="M8 8h8M8 12h8M8 16h5" />
    </svg>
  ),
  romanGalley: (p: React.SVGProps<SVGSVGElement>) => (
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" {...p}>
      <path d="M12 4v11M8 6h8v6H8z" />
      <path d="M3 15c2 3 5 4 9 4s7-1 9-4H3Z" />
      <path d="M7 18l-1.5 3M12 19v2M17 18l1.5 3" />
    </svg>
  ),
  anvilForge: (p: React.SVGProps<SVGSVGElement>) => (
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" {...p}>
      <path d="M7 6h12v4c0 2-2 3-4 3H9c-1.5 0-3-1-5-3l3-4Z" />
      <path d="M10 13v4l-3 3h10l-3-3v-4" />
    </svg>
  ),
  amphoraUrn: (p: React.SVGProps<SVGSVGElement>) => (
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" data-icon="amphoraUrn" {...p}>
      <path d="M9 3h6M10 3v3c-2.5 1.5-4 4-3.5 7 .4 2.5 2 4.5 3.5 5.5V21h4v-2.5c1.5-1 3.1-3 3.5-5.5.5-3-1-5.5-3.5-7V3" />
      <path d="M7.5 6.5C5.5 6.5 4.5 8 5 9.8c.4 1.3 1.4 2 2.5 2.2M16.5 6.5c2 0 3 1.5 2.5 3.3-.4 1.3-1.4 2-2.5 2.2" />
      <path d="M8.5 21h7" />
    </svg>
  ),
  solInvictus: (p: React.SVGProps<SVGSVGElement>) => (
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" data-icon="solInvictus" {...p}>
      <circle cx="12" cy="12" r="4" />
      <path d="M12 2v3.2M12 18.8V22M2 12h3.2M18.8 12H22M4.9 4.9l2.3 2.3M16.8 16.8l2.3 2.3M19.1 4.9l-2.3 2.3M7.2 16.8l-2.3 2.3" />
    </svg>
  ),
  triremeShip: (p: React.SVGProps<SVGSVGElement>) => (
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" data-icon="triremeShip" {...p}>
      <path d="M12 3v11M7.5 5.5c3-.8 6-.8 9 0v6.5c-3-.6-6-.6-9 0V5.5Z" />
      <path d="M2.5 14.5c2 3.2 5.2 4.5 9.5 4.5s7.5-1.3 9.5-4.5l-2-1H4.5l-2 1Z" />
      <path d="M6.5 17.8 5 21M10.2 18.8 9.2 21M13.8 18.8l1 2.2M17.5 17.8 19 21" />
    </svg>
  ),
  crystalGem: (p: React.SVGProps<SVGSVGElement>) => (
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" data-icon="crystalGem" {...p}>
      <path d="M12 2.5 19.5 9 12 21.5 4.5 9 12 2.5Z" />
      <path d="M4.5 9h15M12 2.5 9 9l3 12.5M12 2.5 15 9l-3 12.5" />
    </svg>
  ),
  homeVilla: (p: React.SVGProps<SVGSVGElement>) => (
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" data-icon="homeVilla" {...p}>
      <path d="M3 10.5 12 4l9 6.5M5 9.5V20h14V9.5" />
      <path d="M3 20h18M9.5 20v-6h5v6M8 10h8" />
    </svg>
  ),
  penQuill: (p: React.SVGProps<SVGSVGElement>) => (
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" data-icon="penQuill" {...p}>
      <path d="M20.2 3.8c-4.5 0-8.8 2.4-11.4 6.2L6 14.5 3.5 20.5l6-2.5 4.5-2.8c3.8-2.6 6.2-6.9 6.2-11.4Z" />
      <path d="M14.5 9.5 4 20" />
    </svg>
  ),
};
