searchState.loadedDescShard("bmw_evh", 0, "The BMW Eventhandler crate\nA chunk of data returned by the <code>crate::EventHandler</code>. …\nThe <code>crate::Connection</code> struct represents a connection. It …\nThe <code>crate::EventHandler</code> trait is implemented by the …\nBuilder struct for the crate. All implementations are …\nStatistical information for the <code>crate::EventHandler</code>. This …\nThe <code>crate::UserContext</code> trait is returned on all callbacks …\nThe <code>crate::WriteHandle</code> struct may be used to write data to …\nThe  number of connections accepted by the …\nAdd a client connection to this <code>crate::EventHandler</code>.\nAdd a server connection to this <code>crate::EventHandler</code>.\nBuilds a client side <code>crate::Connection</code> that can be added …\nBuilds a <code>crate::EventHandler</code> with the specified vector of …\nBuilds a server side <code>crate::Connection</code> that can be added …\nThe total number of bytes <code>delay written</code> by the …\nThe total number of bytes read by the <code>crate::EventHandler</code> …\nClear all slabs that are associated with this …\nClear all slabs, through the <code>slab_id</code> specified, that are …\nClose the underlying connection for this <code>crate::WriteHandle</code>…\nThe  number of connections closed by the …\nRetrieves the data associated with this chunk as a <code>slice</code>.\nThe number of delayed writes completed by the …\nDisable the message that is sent by configuring …\nThe  number of event loops that occured by the …\nThe <code>crate::evh!</code> macro builds a <code>crate::EventHandler</code> …\nThe <code>crate::evh_oro!</code> macro builds a <code>crate::EventHandler</code> …\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nGet the <code>user_data</code> object associated with this thread.\nRetrieves the <code>id</code> for this Connection. The id is a unique …\nRetrieve the underlying connection’s id.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nGet the next chunk of data, if available, that has been …\nReturns the <code>origin_id</code> for a <code>crate::Connection</code>. The …\nThe  number of reads completed by the <code>crate::EventHandler</code> …\nSet the OnAccept handler for this <code>crate::EventHandler</code>. …\nSet the OnClose handler for this <code>crate::EventHandler</code>. When …\nSet the OnHousekeeper handler for this <code>crate::EventHandler</code>…\nSets the OnPanic handler for this  <code>crate::EventHandler</code>. …\nSet the OnRead handler for this <code>crate::EventHandler</code>. When …\nSet the <code>user_data</code> object associated with this thread.\nRetrieves the <code>slab_id</code> of the slab for this <code>crate::Chunk</code>. …\nStart the <code>crate::EventHandler</code>. This function must be …\nTrigger a callback of the handler specified by …\nThis function will block until statistical data is ready …\nWrite data to the underlying connection for this …\nReturns a <code>crate::WriteHandle</code> which can be used to write …")