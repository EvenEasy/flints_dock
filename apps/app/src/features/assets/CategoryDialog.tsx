import type { AssetCategory } from '../../../frontend-contract/categories';
import { Dialog } from '../../shared/ui/Dialog';
import { CategoryNotice } from '../../shared/ui/Notice';
const descriptions: Record<string, string> = {
  scam: 'Підозрілі за Jupiter audit.isSus. Це сигнал провайдера, не доказ шахрайства.',
  dust: 'Ненульові активи з малою оціненою вартістю. Це не оцінка вигідності свапу.',
  dead_token:
    'Під час перевірки Jupiter не знайшов маршрут TOKEN → SOL. Це не довічна оцінка вартості.',
  nft: 'Classic, programmable, Core та compressed NFT. Ліквідація не виконується.',
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
            <time>{new Date(Number(item.checkedAt) * 1000).toLocaleString('uk-UA')}</time>
          </li>
        ))}
      </ul>
      {category?.status.status === 'complete' && category.count === 0 && (
        <p>Активів цієї категорії не виявлено.</p>
      )}
      {!category && <p>Результат цієї перевірки недоступний.</p>}
    </Dialog>
  );
}
