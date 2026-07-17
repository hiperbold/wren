// Fake mouse cursor (filled Lucide pointer) for the demo.
export const Cursor: React.FC<{ size?: number; pressed?: boolean }> = ({
  size = 56,
  pressed = false,
}) => (
  <svg
    width={size}
    height={size}
    viewBox="0 0 24 24"
    style={{
      display: "block",
      filter: "drop-shadow(0 4px 8px rgba(0,0,0,0.55))",
      transform: pressed ? "scale(0.82)" : "scale(1)",
      transformOrigin: "top left",
    }}
  >
    <path
      d="M4.037 4.688a.495.495 0 0 1 .651-.651l16 6.5a.5.5 0 0 1-.063.947l-6.124 1.58a2 2 0 0 0-1.438 1.435l-1.579 6.126a.5.5 0 0 1-.947.063z"
      fill="#ffffff"
      stroke="#171513"
      strokeWidth={1.2}
      strokeLinejoin="round"
    />
  </svg>
);
