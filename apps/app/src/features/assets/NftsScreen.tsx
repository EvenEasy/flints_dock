import { media } from '../../shared/assets';
import { ScreenTitle } from '../../shared/ui/ScreenTitle';
import { Icon } from '../../shared/ui/Icon';
import { CategoryNotice, Notice } from '../../shared/ui/Notice';
import { shortAddress } from '../../shared/format';
import type { ScreenProps } from '../../app/ScreenProps';

/** Render verified backend NFT records; no external metadata/image URI is loaded automatically. */
export function NftsScreen({ preview, analysis }: ScreenProps) {
  const classic = analysis?.nfts?.classic.items;
  const core = analysis?.nfts?.core.items;
  return (
    <>
      <ScreenTitle subtitle="Your collectibles are not part of cleanup.">
        NFTs <span className="title-suffix">(VIEW ONLY)</span>
      </ScreenTitle>
      {!preview && (
        <>
          <CategoryNotice status={analysis?.nfts?.status} />
          <CategoryNotice status={analysis?.nfts?.classic.status} />
          <CategoryNotice status={analysis?.nfts?.core.status} />
        </>
      )}
      <div className="nft-grid">
        {preview ? (
          Array.from({ length: 9 }, (_, index) => (
            <figure key={index}>
              <img
                src={media(`nfts/nft-${String(index + 1).padStart(2, '0')}.png`)}
                alt={`Reference collectible ${index + 1}`}
                width="78"
                height="83"
                loading="lazy"
              />
            </figure>
          ))
        ) : (
          <>
            {classic?.map((nft) => (
              <figure className="nft-placeholder" key={nft.mint}>
                <Icon name="nft" tone="purple" />
                <figcaption>
                  <strong>{nft.metadata.name ?? 'Classic NFT'}</strong>
                  <span title={nft.mint}>{shortAddress(nft.mint)}</span>
                  <small>
                    {nft.programmable ? 'Programmable' : 'Classic'}
                    {nft.edition ? ' edition' : ''}
                  </small>
                </figcaption>
              </figure>
            ))}
            {core?.map((nft) => (
              <figure className="nft-placeholder" key={nft.address}>
                <Icon name="nft" tone="purple" />
                <figcaption>
                  <strong>{nft.name || 'Core asset'}</strong>
                  <span title={nft.address}>{shortAddress(nft.address)}</span>
                  <small>MPL Core</small>
                </figcaption>
              </figure>
            ))}
          </>
        )}
      </div>
      {!preview &&
        (!analysis?.nfts ? (
          <p className="empty-state">NFT discovery was not selected.</p>
        ) : classic === null && core === null ? (
          <p className="empty-state">NFT inventory is unavailable.</p>
        ) : (classic?.length ?? 0) + (core?.length ?? 0) === 0 ? (
          <p className="empty-state">No verified classic or Core NFTs found.</p>
        ) : null)}
      <Notice>Compressed NFTs are unavailable and not part of this inventory.</Notice>
      {!preview && analysis?.cnfts && <CategoryNotice status={analysis.cnfts.status} />}
      <p className="nft-count">
        {preview
          ? '7 NFTs SHOWN'
          : `${(classic?.length ?? 0) + (core?.length ?? 0)} AVAILABLE NFTs SHOWN`}
      </p>
      {!preview && (
        <p className="micro-note">
          External artwork is not fetched automatically. Addresses and names come from the backend.
        </p>
      )}
    </>
  );
}
