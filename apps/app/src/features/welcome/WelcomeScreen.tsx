import {
  Brand,
  MascotHero,
  MechanicalPanel,
  SurfaceFrame,
  TitlePedestal,
} from '../../shared/ui/Design';
import { designAsset } from '../../shared/assets';

/** The welcome composition has one connection action and a neutral disconnected indicator. */
export function WelcomeScreen({ onConnect }: { onConnect: () => void }) {
  return (
    <div className="page page--welcome">
      <Brand />
      <MascotHero />
      <TitlePedestal />
      <MechanicalPanel className="connect-module">
        <h2 className="type-section">CONNECT A WALLET</h2>
        <p className="connect-description">
          Connect your Solana wallet
          <br />
          to find unwanted assets and reclaim SOL.
        </p>
        <button className="connect-button" type="button" onClick={onConnect}>
          <SurfaceFrame />
          <span>
            <img src={designAsset('welcome', 'wallet_outline_icon')} alt="" aria-hidden="true" />
            CONNECT WALLET
          </span>
        </button>
        <p className="wallet-status">
          <span className="status-led" aria-hidden="true" />
          WALLET NOT CONNECTED
        </p>
      </MechanicalPanel>
    </div>
  );
}
