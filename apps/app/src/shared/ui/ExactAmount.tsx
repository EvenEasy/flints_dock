/** Preserve every supplied digit; the unit has its own slot and long amounts wrap safely. */
export function ExactAmount({
  amount,
  unit = 'SOL',
  size = 'amount',
}: {
  amount: string;
  unit?: string;
  size?: 'amount' | 'metric' | 'returned' | 'row';
}) {
  return (
    <span className={`exact-amount type-numeric type-numeric--${size}`} title={`${amount} ${unit}`}>
      <span className="exact-digits">{amount}</span> <span className="exact-unit">{unit}</span>
    </span>
  );
}
