use std::rc::Rc;
use std::collections::HashMap;
use std::iter::FromIterator;
use std::path::PathBuf;
use std::string::ToString;
use serde::{Serialize, Deserialize, de::DeserializeOwned};
use crate::convert;
use crate::auth;

use google_datastore1::RunQueryRequest;


pub use crate::auth::Auth;

///////////////////////////////////////////////////////////////////////////////
// HELPERS
///////////////////////////////////////////////////////////////////////////////

pub trait EntityKey {
    fn entity_kind_key() -> String;
    fn entity_name_key(&self) -> String;
}


#[derive(Debug)]
pub enum Error {
    Serialization {
        msg: String,
    },
    Deserialization {
        msg: String,
    },
    DatabaseResponse(google_datastore1::Error),
    NoPayload,
}

unsafe impl Send for Error {}


///////////////////////////////////////////////////////////////////////////////
// CLIENT
///////////////////////////////////////////////////////////////////////////////

type Handle = google_datastore1::Datastore<hyper::Client, auth::Auth>;

#[derive(Clone)]
pub struct DatastoreClient {
    handle: Rc<Handle>,
    project_id: String,
}

impl DatastoreClient {
    /// Automatically finds auth credentials.
    /// See `Auth::new()` for auth related details.
    pub fn new() -> Result<Self, String> {
        let auth = Auth::new()?;
        DatastoreClient::new_with_auth(auth)
    }
    pub fn new_with_auth(auth: Auth) -> Result<Self, String> {
        let project_id = auth.project_id.clone();
        let client = hyper::Client::with_connector(
            hyper::net::HttpsConnector::new(hyper_rustls::TlsClient::new())
        );
        let hub = google_datastore1::Datastore::new(client, auth);
        Ok(DatastoreClient {
            handle: Rc::new(hub),
            project_id,
        })
    }
    pub fn insert<T: Serialize + EntityKey>(&self, value: T) -> Result<(), Error> {
        let kind_key = T::entity_kind_key();
        let name_key = value.entity_name_key();
        let properties = convert::to_datastore_value(value)
            .and_then(|value| {
                value.entity_value
            })
            .and_then(|x| x.properties)
            .ok_or(Error::Serialization {
                msg: String::from("expecting struct/map like input")
            })?;
        let entity = google_datastore1::Entity {
            properties: Some(properties),
            key: Some(google_datastore1::Key {
                path: Some(vec![
                    google_datastore1::PathElement {
                        kind: Some(kind_key.to_owned()),
                        name: Some(name_key.to_owned()),
                        id: None
                    }
                ]),
                partition_id: None
            })
        };
        let req = google_datastore1::CommitRequest {
            transaction: None,
            mutations: Some(vec![
                google_datastore1::Mutation {
                    insert: Some(entity),
                    delete: None,
                    update: None,
                    base_version: None,
                    upsert: None
                }
            ]),
            mode: Some(String::from("NON_TRANSACTIONAL"))
        };
        let result = self.handle
            .projects()
            .commit(req, &self.project_id)
            .doit();
        match result {
            Ok(_) => Ok(()),
            Err(e) => Err(Error::DatabaseResponse(e))
        }
    }
    pub fn upsert<T: Serialize + EntityKey>(&self, value: T) -> Result<(), Error> {
        let kind_key = T::entity_kind_key();
        let name_key = value.entity_name_key();
        let properties = convert::to_datastore_value(value)
            .and_then(|value| {
                value.entity_value
            })
            .and_then(|x| x.properties)
            .ok_or(Error::Serialization {
                msg: String::from("expecting struct/map like input")
            })?;
        let entity = google_datastore1::Entity {
            properties: Some(properties),
            key: Some(google_datastore1::Key {
                path: Some(vec![
                    google_datastore1::PathElement {
                        kind: Some(kind_key.to_owned()),
                        name: Some(name_key.to_owned()),
                        id: None
                    }
                ]),
                partition_id: None
            })
        };
        let req = google_datastore1::CommitRequest {
            transaction: None,
            mutations: Some(vec![
                google_datastore1::Mutation {
                    insert: None,
                    delete: None,
                    update: None,
                    base_version: None,
                    upsert: Some(entity),
                }
            ]),
            mode: Some(String::from("NON_TRANSACTIONAL"))
        };
        let result = self.handle
            .projects()
            .commit(req, &self.project_id)
            .doit();
        match result {
            Ok(_) => Ok(()),
            Err(e) => Err(Error::DatabaseResponse(e))
        }
    }
    pub fn update<T: Serialize + EntityKey>(&self, value: T) -> Result<(), Error> {
        let kind_key = T::entity_kind_key();
        let name_key = value.entity_name_key();
        let properties = convert::to_datastore_value(value)
            .and_then(|value| {
                value.entity_value
            })
            .and_then(|x| x.properties)
            .ok_or(Error::Serialization {
                msg: String::from("expecting struct/map like input")
            })?;
        let entity = google_datastore1::Entity {
            properties: Some(properties),
            key: Some(google_datastore1::Key {
                path: Some(vec![
                    google_datastore1::PathElement {
                        kind: Some(kind_key.to_owned()),
                        name: Some(name_key.to_owned()),
                        id: None
                    }
                ]),
                partition_id: None
            })
        };
        let req = google_datastore1::CommitRequest {
            transaction: None,
            mutations: Some(vec![
                google_datastore1::Mutation {
                    insert: None,
                    delete: None,
                    update: Some(entity),
                    base_version: None,
                    upsert: None,
                }
            ]),
            mode: Some(String::from("NON_TRANSACTIONAL"))
        };
        let result = self.handle
            .projects()
            .commit(req, &self.project_id)
            .doit();
        match result {
            Ok(_) => Ok(()),
            Err(e) => Err(Error::DatabaseResponse(e))
        }
    }
    pub fn get<T: DeserializeOwned + EntityKey, K: ToString>(&self, name_key: K) -> Result<T, Error> {
        let kind_key = T::entity_kind_key();
        let req = google_datastore1::LookupRequest {
            keys: Some(vec![
                google_datastore1::Key {
                    path: Some(vec![
                        google_datastore1::PathElement {
                            kind: Some(kind_key),
                            name: Some(name_key.to_string()),
                            id: None
                        }
                    ]),
                    partition_id: None
                }]),
            read_options: None
        };
        let result = self.handle
            .projects()
            .lookup(req, &self.project_id)
            .doit();
        match result {
            Ok((_, lookup_response)) => {
                let payload = lookup_response.found
                    .and_then(|entities| {
                        entities.first().map(|x| x.clone())
                    })
                    .and_then(|x| x.entity)
                    .ok_or(Error::NoPayload)?;
                convert::from_datastore_entity(payload.clone())
                    .ok_or_else(|| {
                        Error::Deserialization {
                            msg: String::from("conversion or parser error")
                        }
                    })
            }
            Err(e) => Err(Error::DatabaseResponse(e)),
        }
    }
    pub fn list<T: DeserializeOwned + EntityKey>(&self) -> Result<Vec<T>, Error> {
        let kind_key = T::entity_kind_key();
        let mut query = RunQueryRequest{
            query: Some(google_datastore1::Query{
                start_cursor: None,
                kind: Some(vec![ google_datastore1::KindExpression { name: Some(kind_key)} ]),
                projection: None,
                distinct_on: None,
                filter: None,
                limit: None,
                offset: None,
                end_cursor:None,
                order: None,
            }),
            partition_id: None,
            gql_query: None,
            read_options: None,
        };

        let result = self.handle
            .projects()
            //.lookup(req, &self.project_id)
            .run_query(query, &self.project_id)
            .doit();

        match result {
            Ok((_, query_response)) => {
                let payload = query_response.batch
                    .and_then(|batch| batch.entity_results )
                    .and_then(|entities| {
                        Some(entities.into_iter().filter_map(|x| x.entity)
                        .filter_map(|x| convert::from_datastore_entity(x.clone()))
                        .collect::<Vec<T>>())
                    })
                    .ok_or(Error::NoPayload)?;
                    Ok(payload)
            }
            Err(e) => Err(Error::DatabaseResponse(e)),
        }
    }
    pub fn delete<T: EntityKey, K: ToString>(&self, name_key: K) -> Result<(), Error> {
        let kind_key = T::entity_kind_key();
        let name_key = name_key.to_string();
        let entity_key = google_datastore1::Key {
            path: Some(vec![
                google_datastore1::PathElement {
                    kind: Some(kind_key.to_owned()),
                    name: Some(name_key.to_owned()),
                    id: None
                }
            ]),
            partition_id: None
        };
        let req = google_datastore1::CommitRequest {
            transaction: None,
            mutations: Some(vec![
                google_datastore1::Mutation {
                    insert: None,
                    delete: Some(entity_key),
                    update: None,
                    base_version: None,
                    upsert: None,
                }
            ]),
            mode: Some(String::from("NON_TRANSACTIONAL"))
        };
        let result = self.handle
            .projects()
            .commit(req, &self.project_id)
            .doit();
        match result {
            Ok(_) => Ok(()),
            Err(e) => Err(Error::DatabaseResponse(e))
        }
    }
}
