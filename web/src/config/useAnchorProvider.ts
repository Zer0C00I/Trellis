import { AnchorProvider, Program } from '@coral-xyz/anchor'
import { useAnchorWallet, useConnection } from '@solana/wallet-adapter-react'
import { useMemo } from 'react'
import { PROGRAM_ID } from './solana'
import { FamilyVault, IDL } from '../idl/family_vault'

export function useAnchorProvider() {
  const { connection } = useConnection()
  const wallet = useAnchorWallet()

  const provider = useMemo(() => {
    if (!wallet) return null
    return new AnchorProvider(connection, wallet, {
      commitment: 'confirmed',
      preflightCommitment: 'confirmed',
    })
  }, [connection, wallet])

  const program = useMemo(() => {
    if (!provider) return null
    // IDL is generated from the real contract (target/idl/family_vault.json),
    // so account order and writable/signer flags always match the program.
    return new Program<FamilyVault>({ ...IDL, address: PROGRAM_ID.toBase58() } as FamilyVault, provider)
  }, [provider])

  return { provider, program }
}
