import React from 'react';

/**
 * React Bits — StarBorder (adapted for overlay use).
 *
 * Source: reactbits.dev (jsrepo registry `StarBorder-TS-TW`, dependencies: []).
 * The original version is a <button> with two radial blobs orbiting the edges
 * (star-movement-top/bottom keyframes) and a dark inner box. In Wren we reuse
 * only the border animation as a momentary HIGHLIGHT over the Card of the
 * just-activated model — feedback, not permanent decoration. The `overlay` mode
 * renders only the animated layer (absolute, pointer-events-none) over the
 * relative parent; the default mode keeps the original React Bits button.
 */
type StarBorderProps<T extends React.ElementType> =
  React.ComponentPropsWithoutRef<T> & {
    as?: T;
    className?: string;
    children?: React.ReactNode;
    color?: string;
    speed?: React.CSSProperties['animationDuration'];
    thickness?: number;
    /** overlay: only the animated glow layer, over the relative parent element. */
    overlay?: boolean;
  };

const StarBorder = <T extends React.ElementType = 'button'>({
  as,
  className = '',
  color = 'white',
  speed = '6s',
  thickness = 1,
  overlay = false,
  children,
  ...rest
}: StarBorderProps<T>) => {
  const blobs = (
    <>
      <div
        className="absolute bottom-[-11px] right-[-250%] z-0 h-[50%] w-[300%] animate-star-movement-bottom rounded-full opacity-70"
        style={{
          background: `radial-gradient(circle, ${color}, transparent 10%)`,
          animationDuration: speed,
        }}
      />
      <div
        className="absolute left-[-250%] top-[-10px] z-0 h-[50%] w-[300%] animate-star-movement-top rounded-full opacity-70"
        style={{
          background: `radial-gradient(circle, ${color}, transparent 10%)`,
          animationDuration: speed,
        }}
      />
    </>
  );

  if (overlay) {
    return (
      <div
        aria-hidden
        className={`pointer-events-none absolute inset-0 z-0 overflow-hidden rounded-lg ${className}`}
      >
        {blobs}
      </div>
    );
  }

  const Component = as || 'button';
  return (
    <Component
      className={`relative inline-block overflow-hidden rounded-[20px] ${className}`}
      {...(rest as Record<string, unknown>)}
      style={{
        padding: `${thickness}px 0`,
        ...(rest as { style?: React.CSSProperties }).style,
      }}
    >
      {blobs}
      <div className="relative z-1 rounded-[20px] border border-gray-800 bg-gradient-to-b from-black to-gray-900 px-[26px] py-[16px] text-center text-[16px] text-white">
        {children}
      </div>
    </Component>
  );
};

export default StarBorder;
