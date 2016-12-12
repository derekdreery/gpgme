#![allow(trivial_numeric_casts)]
use std::ffi::CStr;
use std::marker::PhantomData;
use std::result;
use std::str::Utf8Error;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use libc;
use ffi;

use {Context, Error, HashAlgorithm, ImportFlags, KeyAlgorithm, OpResult, Result, SignMode,
     SignatureSummary, Validity};
use error;
use notation::SignatureNotations;

macro_rules! impl_result {
    ($Name:ident: $T:ty = $Constructor:path) => {
        #[derive(Debug)]
        pub struct $Name($T);

        impl Drop for $Name {
            #[inline]
            fn drop(&mut self) {
                unsafe {
                    ffi::gpgme_result_unref(self.0 as *mut libc::c_void);
                }
            }
        }

        impl Clone for $Name {
            #[inline]
            fn clone(&self) -> $Name {
                unsafe {
                    ffi::gpgme_result_ref(self.0 as *mut libc::c_void);
                    $Name(self.0)
                }
            }
        }

        unsafe impl OpResult for $Name {
            fn from_context(ctx: &Context) -> Option<$Name> {
                unsafe {
                    $Constructor(ctx.as_raw()).as_mut().map(|r| {
                        ffi::gpgme_result_ref(r as *mut _ as *mut libc::c_void);
                        $Name::from_raw(r)
                    })
                }
            }
        }

        impl $Name {
            impl_wrapper!($Name: $T);
        }
    };
}

macro_rules! impl_subresult {
    ($Name:ident: $T:ty, $IterName:ident, $Owner:ty) => {
        #[derive(Debug, Copy, Clone)]
        pub struct $Name<'a>($T, PhantomData<&'a $Owner>);

        impl<'a> $Name<'a> {
            impl_wrapper!(@phantom $Name: $T);
        }

        #[derive(Debug, Copy, Clone)]
        pub struct $IterName<'a> {
            current: $T,
            left: Option<usize>,
            phantom: PhantomData<&'a $Owner>,
        }

        impl<'a> $IterName<'a> {
            pub unsafe fn from_list(first: $T) -> Self {
                let left = count_list!(first);
                $IterName { current: first, left: left, phantom: PhantomData }
            }
        }

        impl<'a> Iterator for $IterName<'a> {
            list_iterator!($Name<'a>, $Name::from_raw);
        }
    };
}

impl_subresult!(InvalidKey: ffi::gpgme_invalid_key_t, InvalidKeys, ());

impl<'a> InvalidKey<'a> {
    #[inline]
    pub fn fingerprint(&self) -> result::Result<&'a str, Option<Utf8Error>> {
        self.fingerprint_raw().map_or(Err(None), |s| s.to_str().map_err(Some))
    }

    #[inline]
    pub fn fingerprint_raw(&self) -> Option<&'a CStr> {
        unsafe { (*self.0).fpr.as_ref().map(|s| CStr::from_ptr(s)) }
    }

    #[inline]
    pub fn reason(&self) -> Option<Error> {
        unsafe {
            match (*self.0).reason {
                error::GPG_ERR_NO_ERROR => None,
                e => Some(Error::new(e)),
            }
        }
    }
}

impl_result!(KeyListResult: ffi::gpgme_keylist_result_t = ffi::gpgme_op_keylist_result);
impl KeyListResult {
    pub fn is_truncated(&self) -> bool {
        unsafe { (*self.0).truncated() }
    }
}

impl_result!(KeyGenerationResult: ffi::gpgme_genkey_result_t = ffi::gpgme_op_genkey_result);
impl KeyGenerationResult {
    #[inline]
    pub fn has_primary_key(&self) -> bool {
        unsafe { (*self.0).primary() }
    }

    #[inline]
    pub fn has_sub_key(&self) -> bool {
        unsafe { (*self.0).sub() }
    }

    #[inline]
    pub fn has_uid(&self) -> bool {
        unsafe { (*self.0).uid() }
    }

    #[inline]
    pub fn fingerprint(&self) -> result::Result<&str, Option<Utf8Error>> {
        self.fingerprint_raw().map_or(Err(None), |s| s.to_str().map_err(Some))
    }

    #[inline]
    pub fn fingerprint_raw(&self) -> Option<&CStr> {
        unsafe { (*self.0).fpr.as_ref().map(|s| CStr::from_ptr(s)) }
    }
}


impl_result!(ImportResult: ffi::gpgme_import_result_t = ffi::gpgme_op_import_result);
impl ImportResult {
    #[inline]
    pub fn considered(&self) -> u32 {
        unsafe { (*self.0).considered as u32 }
    }

    #[inline]
    pub fn without_user_id(&self) -> u32 {
        unsafe { (*self.0).no_user_id as u32 }
    }

    #[inline]
    pub fn imported(&self) -> u32 {
        unsafe { (*self.0).imported as u32 }
    }

    #[inline]
    pub fn imported_rsa(&self) -> u32 {
        unsafe { (*self.0).imported_rsa as u32 }
    }

    #[inline]
    pub fn unchanged(&self) -> u32 {
        unsafe { (*self.0).unchanged as u32 }
    }

    #[inline]
    pub fn new_user_ids(&self) -> u32 {
        unsafe { (*self.0).new_user_ids as u32 }
    }

    #[inline]
    pub fn new_subkeys(&self) -> u32 {
        unsafe { (*self.0).new_sub_keys as u32 }
    }

    #[inline]
    pub fn new_signatures(&self) -> u32 {
        unsafe { (*self.0).new_signatures as u32 }
    }

    #[inline]
    pub fn new_revocations(&self) -> u32 {
        unsafe { (*self.0).new_revocations as u32 }
    }

    #[inline]
    pub fn secret_considered(&self) -> u32 {
        unsafe { (*self.0).secret_read as u32 }
    }

    #[inline]
    pub fn secret_imported(&self) -> u32 {
        unsafe { (*self.0).secret_imported as u32 }
    }

    #[inline]
    pub fn secret_unchanged(&self) -> u32 {
        unsafe { (*self.0).secret_unchanged as u32 }
    }

    #[inline]
    pub fn not_imported(&self) -> u32 {
        unsafe { (*self.0).not_imported as u32 }
    }

    #[inline]
    pub fn imports(&self) -> Imports {
        unsafe { Imports::from_list((*self.0).imports) }
    }
}

impl_subresult!(Import: ffi::gpgme_import_status_t, Imports, ImportResult);
impl<'a> Import<'a> {
    #[inline]
    pub fn fingerprint(&self) -> result::Result<&'a str, Option<Utf8Error>> {
        self.fingerprint_raw().map_or(Err(None), |s| s.to_str().map_err(Some))
    }

    #[inline]
    pub fn fingerprint_raw(&self) -> Option<&'a CStr> {
        unsafe { (*self.0).fpr.as_ref().map(|s| CStr::from_ptr(s)) }
    }

    #[inline]
    pub fn result(&self) -> Result<()> {
        unsafe {
            return_err!((*self.0).result);
            Ok(())
        }
    }

    #[inline]
    pub fn status(&self) -> ImportFlags {
        unsafe { ImportFlags::from_bits_truncate((*self.0).status) }
    }
}

impl_result!(EncryptionResult: ffi::gpgme_encrypt_result_t = ffi::gpgme_op_encrypt_result);
impl EncryptionResult {
    #[inline]
    pub fn invalid_recipients(&self) -> InvalidKeys {
        unsafe { InvalidKeys::from_list((*self.0).invalid_recipients) }
    }
}
impl_result!(DecryptionResult: ffi::gpgme_decrypt_result_t = ffi::gpgme_op_decrypt_result);
impl DecryptionResult {
    #[inline]
    pub fn unsupported_algorithm(&self) -> result::Result<&str, Option<Utf8Error>> {
        self.unsupported_algorithm_raw().map_or(Err(None), |s| s.to_str().map_err(Some))
    }

    #[inline]
    pub fn unsupported_algorithm_raw(&self) -> Option<&CStr> {
        unsafe { (*self.0).unsupported_algorithm.as_ref().map(|s| CStr::from_ptr(s)) }
    }

    #[inline]
    pub fn is_wrong_key_usage(&self) -> bool {
        unsafe { (*self.0).wrong_key_usage() }
    }

    #[inline]
    pub fn filename(&self) -> result::Result<&str, Option<Utf8Error>> {
        self.filename_raw().map_or(Err(None), |s| s.to_str().map_err(Some))
    }

    #[inline]
    pub fn filename_raw(&self) -> Option<&CStr> {
        unsafe { (*self.0).file_name.as_ref().map(|s| CStr::from_ptr(s)) }
    }

    #[inline]
    pub fn recipients(&self) -> Recipients {
        unsafe { Recipients::from_list((*self.0).recipients) }
    }
}

impl_subresult!(Recipient: ffi::gpgme_recipient_t,
                Recipients,
                DecryptionResult);
impl<'a> Recipient<'a> {
    #[inline]
    pub fn key_id(&self) -> result::Result<&'a str, Option<Utf8Error>> {
        self.key_id_raw().map_or(Err(None), |s| s.to_str().map_err(Some))
    }

    #[inline]
    pub fn key_id_raw(&self) -> Option<&'a CStr> {
        unsafe { (*self.0).keyid.as_ref().map(|s| CStr::from_ptr(s)) }
    }

    #[inline]
    pub fn algorithm(&self) -> KeyAlgorithm {
        unsafe { KeyAlgorithm::from_raw((*self.0).pubkey_algo) }
    }

    #[inline]
    pub fn status(&self) -> Result<()> {
        unsafe {
            return_err!((*self.0).status);
            Ok(())
        }
    }
}

impl_result!(SigningResult: ffi::gpgme_sign_result_t = ffi::gpgme_op_sign_result);
impl SigningResult {
    #[inline]
    pub fn invalid_signers(&self) -> InvalidKeys {
        unsafe { InvalidKeys::from_list((*self.0).invalid_signers) }
    }

    #[inline]
    pub fn new_signatures(&self) -> NewSignatures {
        unsafe { NewSignatures::from_list((*self.0).signatures) }
    }
}

impl_subresult!(NewSignature: ffi::gpgme_new_signature_t,
                NewSignatures,
                SigningResult);
impl<'a> NewSignature<'a> {
    #[inline]
    pub fn fingerprint(&self) -> result::Result<&'a str, Option<Utf8Error>> {
        self.fingerprint_raw().map_or(Err(None), |s| s.to_str().map_err(Some))
    }

    #[inline]
    pub fn fingerprint_raw(&self) -> Option<&'a CStr> {
        unsafe { (*self.0).fpr.as_ref().map(|s| CStr::from_ptr(s)) }
    }

    #[inline]
    pub fn creation_time(&self) -> SystemTime {
        let timestamp = unsafe { (*self.0).timestamp };
        UNIX_EPOCH + Duration::from_secs(timestamp as u64)
    }

    #[inline]
    pub fn mode(&self) -> SignMode {
        unsafe { SignMode::from_raw((*self.0).sig_type) }
    }

    #[inline]
    pub fn key_algorithm(&self) -> KeyAlgorithm {
        unsafe { KeyAlgorithm::from_raw((*self.0).pubkey_algo) }
    }

    #[inline]
    pub fn hash_algorithm(&self) -> HashAlgorithm {
        unsafe { HashAlgorithm::from_raw((*self.0).hash_algo) }
    }

    #[inline]
    pub fn signature_class(&self) -> u32 {
        unsafe { (*self.0).sig_class as u32 }
    }
}

impl_result!(VerificationResult: ffi::gpgme_verify_result_t = ffi::gpgme_op_verify_result);
impl VerificationResult {
    #[inline]
    pub fn filename(&self) -> result::Result<&str, Option<Utf8Error>> {
        self.filename_raw().map_or(Err(None), |s| s.to_str().map_err(Some))
    }

    #[inline]
    pub fn filename_raw(&self) -> Option<&CStr> {
        unsafe { (*self.0).file_name.as_ref().map(|s| CStr::from_ptr(s)) }
    }

    #[inline]
    pub fn signatures(&self) -> Signatures {
        unsafe { Signatures::from_list((*self.0).signatures) }
    }
}

ffi_enum_wrapper! {
    pub enum PkaTrust: libc::c_uint {
        Unknown = 0,
        Bad = 1,
        Okay = 2,
    }
}

impl_subresult!(Signature: ffi::gpgme_signature_t,
                Signatures,
                VerificationResult);
impl<'a> Signature<'a> {
    #[inline]
    pub fn summary(&self) -> SignatureSummary {
        unsafe { SignatureSummary::from_bits_truncate((*self.0).summary as u32) }
    }

    #[inline]
    pub fn fingerprint(&self) -> result::Result<&'a str, Option<Utf8Error>> {
        self.fingerprint_raw().map_or(Err(None), |s| s.to_str().map_err(Some))
    }

    #[inline]
    pub fn fingerprint_raw(&self) -> Option<&'a CStr> {
        unsafe { (*self.0).fpr.as_ref().map(|s| CStr::from_ptr(s)) }
    }

    #[inline]
    pub fn status(&self) -> Result<()> {
        unsafe {
            return_err!((*self.0).status);
            Ok(())
        }
    }

    #[inline]
    pub fn creation_time(&self) -> Option<SystemTime> {
        let timestamp = unsafe { (*self.0).timestamp };
        if timestamp > 0 {
            Some(UNIX_EPOCH + Duration::from_secs(timestamp.into()))
        } else {
            None
        }
    }

    #[inline]
    pub fn expiration_time(&self) -> Option<SystemTime> {
        let expires = unsafe { (*self.0).exp_timestamp };
        if expires > 0 {
            Some(UNIX_EPOCH + Duration::from_secs(expires.into()))
        } else {
            None
        }
    }

    #[inline]
    pub fn never_expires(&self) -> bool {
        self.expiration_time().is_none()
    }

    #[inline]
    pub fn is_wrong_key_usage(&self) -> bool {
        unsafe { (*self.0).wrong_key_usage() }
    }

    #[inline]
    pub fn verified_by_chain(&self) -> bool {
        unsafe { (*self.0).chain_model() }
    }

    #[inline]
    pub fn pka_trust(&self) -> PkaTrust {
        unsafe { PkaTrust::from_raw((*self.0).pka_trust()) }
    }

    #[inline]
    pub fn pka_address(&self) -> result::Result<&'a str, Option<Utf8Error>> {
        self.pka_address_raw().map_or(Err(None), |s| s.to_str().map_err(Some))
    }

    #[inline]
    pub fn pka_address_raw(&self) -> Option<&'a CStr> {
        unsafe { (*self.0).pka_address.as_ref().map(|s| CStr::from_ptr(s)) }
    }

    #[inline]
    pub fn validity(&self) -> Validity {
        unsafe { Validity::from_raw((*self.0).validity) }
    }

    #[inline]
    pub fn nonvalidity_reason(&self) -> Option<Error> {
        unsafe {
            let reason = (*self.0).validity_reason;
            if reason != error::GPG_ERR_NO_ERROR {
                Some(Error::new(reason))
            } else {
                None
            }
        }
    }

    #[inline]
    pub fn key_algorithm(&self) -> KeyAlgorithm {
        unsafe { KeyAlgorithm::from_raw((*self.0).pubkey_algo) }
    }

    #[inline]
    pub fn hash_algorithm(&self) -> HashAlgorithm {
        unsafe { HashAlgorithm::from_raw((*self.0).hash_algo) }
    }

    #[inline]
    pub fn policy_url(&self) -> result::Result<&'a str, Option<Utf8Error>> {
        self.policy_url_raw().map_or(Err(None), |s| s.to_str().map_err(Some))
    }

    #[inline]
    pub fn policy_url_raw(&self) -> Option<&'a CStr> {
        unsafe {
            let mut notation = (*self.0).notations;
            while !notation.is_null() {
                if (*notation).name.is_null() {
                    return (*notation).value.as_ref().map(|s| CStr::from_ptr(s));
                }
                notation = (*notation).next;
            }
            None
        }
    }

    #[inline]
    pub fn notations(&self) -> SignatureNotations<'a, VerificationResult> {
        unsafe { SignatureNotations::from_list((*self.0).notations) }
    }

    #[inline]
    #[cfg(feature = "v1_7_0")]
    pub fn key(&self) -> Option<::Key> {
        unsafe {
            (*self.0).key.as_mut().map(|k| {
                ffi::gpgme_key_ref(k);
                ::Key::from_raw(k)
            })
        }
    }
}
