import React from 'react';
import { Config } from './providers/config/model';
import { ConfigProvider } from './providers/config/provider';
import { NotificationsProvider } from './providers/notifications/provider';
import { ExtensionProvider } from './providers/extension/provider';
import { APIProvider } from './providers/api/provider';
import { BlockHeaderProvider } from './providers/blockHeader/provider';
import { ContractEventsProvider } from './providers/contractEvents/provider';

type InkConfig = {
  config?: Config;
  children?: React.ReactNode;
};

const InkProvider: React.FC<InkConfig> = ({ children, config }) => {
  return (
    <ConfigProvider config={config}>
      <ExtensionProvider>
        <APIProvider>
          <BlockHeaderProvider>
            <ContractEventsProvider>
              <NotificationsProvider>{children}</NotificationsProvider>
            </ContractEventsProvider>
          </BlockHeaderProvider>
        </APIProvider>
      </ExtensionProvider>
    </ConfigProvider>
  );
};

export default InkProvider;
