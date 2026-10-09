import type { AssetCategory } from '../../../frontend-contract/categories';
import { Dialog } from '../../shared/ui/Dialog';
import { CategoryNotice } from '../../shared/ui/Notice';
const descriptions: Record<string, string> = {
  scam: 'Flagged as suspicious by Jupiter audit.isSus. A provider signal, not proof of fraud.',
  dust: 'Nonzero holdings with a low estimated value. Swapping may still cost more than it returns.',
  dead_token:
    'Jupiter found no TOKEN → SOL route when checked. This does not mean the asset is permanently worthless.',
  nft: 'Classic, programmable, Core and compressed NFTs. NFT liquidation is not supported.',
};
export function CategoryDialog({
  name,
  category,
  onClose,
}: {
  name: string;
  category: AssetCategory | undefined;
  onClose: () => void;
}) {
  return (
    <Dialog title={name === 'dead_token' ? 'DEAD TOKEN' : name.toUpperCase()} onClose={onClose}>
      <p className="type-caption">{descriptions[name]}</p>
      <CategoryNotice status={category?.status} />
      <ul className="category-results">
        {category?.items.map((item) => (
          <li key={item.id}>
            <strong>{item.name}</strong>
            <code title={item.mint ?? item.id}>{item.mint ?? item.id}</code>
            <p>
              {item.kind} · {item.valuation} · {item.tradability}
            </p>
            {item.evidence && <p>{item.evidence}</p>}
            {item.risk?.reasons.map((reason) => (
              <p key={reason}>{reason}</p>
            ))}
            <time>{new Date(Number(item.checkedAt) * 1000).toLocaleString('en-US')}</time>
          </li>
        ))}
      </ul>
      {category?.status.status === 'complete' && category.count === 0 && (
        <p>No assets found in this category.</p>
      )}
      {!category && <p>Results for this check are unavailable.</p>}
    </Dialog>
  );
}
