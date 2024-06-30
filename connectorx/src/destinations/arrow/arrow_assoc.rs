use super::{
    errors::{ArrowDestinationError, Result},
    typesystem::{DateTimeWrapperMicro, NaiveDateTimeWrapperMicro, NaiveTimeWrapperMicro},
};
use crate::constants::SECONDS_IN_DAY;
use arrow::{array::{
    ArrayBuilder, BooleanBuilder, BufferBuilder, Date32Builder, Float32Builder, Float64Builder, GenericListArray, GenericListBuilder, Int32Builder, Int64Builder, LargeBinaryBuilder, ListBuilder, PrimitiveBuilder, StringBuilder, Time64MicrosecondBuilder, Time64NanosecondBuilder, TimestampMicrosecondBuilder, TimestampNanosecondBuilder, UInt32Builder, UInt64Builder
}, datatypes::Int32Type, ipc::BoolBuilder};
use arrow::datatypes::Field;
use arrow::datatypes::{DataType as ArrowDataType, TimeUnit};
use chrono::{DateTime, NaiveDate, NaiveDateTime, NaiveTime, Timelike, Utc};
use fehler::throws;
use std::sync::Arc;

/// Associate arrow builder with native type
pub trait ArrowAssoc {
    type Builder: ArrayBuilder + Send;

    fn builder(nrows: usize) -> Self::Builder;
    fn append(builder: &mut Self::Builder, value: Self) -> Result<()>;
    fn field(header: &str) -> Field;
}

macro_rules! impl_arrow_assoc {
    ($T:ty, $AT:expr, $B:ty) => {
        impl ArrowAssoc for $T {
            type Builder = $B;

            fn builder(nrows: usize) -> Self::Builder {
                Self::Builder::with_capacity(nrows)
            }

            #[throws(ArrowDestinationError)]
            fn append(builder: &mut Self::Builder, value: Self) {
                builder.append_value(value);
            }

            fn field(header: &str) -> Field {
                Field::new(header, $AT, false)
            }
        }

        impl ArrowAssoc for Option<$T> {
            type Builder = $B;

            fn builder(nrows: usize) -> Self::Builder {
                Self::Builder::with_capacity(nrows)
            }

            #[throws(ArrowDestinationError)]
            fn append(builder: &mut Self::Builder, value: Self) {
                builder.append_option(value);
            }

            fn field(header: &str) -> Field {
                Field::new(header, $AT, true)
            }
        }
    };
}

impl_arrow_assoc!(u32, ArrowDataType::UInt32, UInt32Builder);
impl_arrow_assoc!(u64, ArrowDataType::UInt64, UInt64Builder);
impl_arrow_assoc!(i32, ArrowDataType::Int32, Int32Builder);
impl_arrow_assoc!(i64, ArrowDataType::Int64, Int64Builder);
impl_arrow_assoc!(f32, ArrowDataType::Float32, Float32Builder);
impl_arrow_assoc!(f64, ArrowDataType::Float64, Float64Builder);
impl_arrow_assoc!(bool, ArrowDataType::Boolean, BooleanBuilder);

macro_rules! impl_arrow_assoc_vec {
    ($T:ty, $AT:expr, $B:ty) => {
        impl ArrowAssoc for Vec<$T> {
            type Builder = $B;

            fn builder(nrows: usize) -> Self::Builder {
                Self::Builder::with_capacity(nrows)
            }

            //#[throws(ArrowDestinationError)]
            fn append(builder: &mut Self::Builder, value: Self) -> Result<()> {
                let val: Vec<Option<$T>> = value.into_iter().map(|v| Some(v)).collect();
                //builder.try_push(Some(val)).unwrap();
                for n in val {
                    builder.append_option(n);
                }
                Ok(())
            }

            fn field(header: &str) -> Field {
                Field::new(
                    header,
                    ArrowDataType::LargeList(Arc::new(Field::new("", $AT, false))),
                    false,
                )
            }
        }

        impl ArrowAssoc for Option<Vec<$T>> {
            type Builder = $B;

            fn builder(nrows: usize) -> Self::Builder {
                Self::Builder::with_capacity(nrows)
            }

            //#[throws(ArrowDestinationError)]
            fn append(builder: &mut Self::Builder, value: Self) -> Result<()> {
                match value {
                    Some(values) => {
                        let val: Vec<Option<$T>> = values.into_iter().map(|v| Some(v)).collect();
                        for n in val {
                            builder.append_option(n);
                        }
                        Ok(())
                    }
                    None => {
                        builder.append_null();
                        Ok(())
                    },
                }
            }

            fn field(header: &str) -> Field {
                Field::new(
                    header,
                    ArrowDataType::LargeList(Arc::new(Field::new("", $AT, false))),
                    true,
                )
            }
        }
    };
}

macro_rules! impl_arrow_assoc_primitive_vec {
    ($T:ty, $AT:expr) => {
        impl_arrow_assoc_vec!($T, $AT, $T);
    };
}

// impl_arrow_assoc_vec!(bool, ArrowDataType::Boolean, BooleanBuilder);
impl_arrow_assoc_vec!(i32, ArrowDataType::Int32, Int32Builder);
impl_arrow_assoc_vec!(i64, ArrowDataType::Int64, Int64Builder);
// impl_arrow_assoc_vec!(u32, ArrowDataType::UInt32, UInt32Builder);
// impl_arrow_assoc_vec!(u64, ArrowDataType::UInt64, UInt64Builder);
// impl_arrow_assoc_vec!(f32, ArrowDataType::Float32, Float32Builder);
// impl_arrow_assoc_vec!(f64, ArrowDataType::Float64, Float64Builder);

// impl_arrow_assoc_vec!(bool, GenericListArray<i32>, ArrowDataType::Boolean);
// impl_arrow_assoc_vec!(i32, GenericListArray<i32>, ArrowDataType::Int32);
// impl_arrow_assoc_vec!(i64, GenericListArray<i32>, ArrowDataType::Int64);
// impl_arrow_assoc_vec!(u32, GenericListArray<i32>, ArrowDataType::UInt32);
// impl_arrow_assoc_vec!(u64, GenericListArray<i32>, ArrowDataType::UInt64);
// impl_arrow_assoc_vec!(f32, GenericListArray<i32>, ArrowDataType::Float32);
// impl_arrow_assoc_vec!(f64, GenericListArray<i32>, ArrowDataType::Float64);

impl ArrowAssoc for &str {
    type Builder = StringBuilder;

    fn builder(nrows: usize) -> Self::Builder {
        StringBuilder::with_capacity(1024, nrows)
    }

    #[throws(ArrowDestinationError)]
    fn append(builder: &mut Self::Builder, value: Self) {
        builder.append_value(value);
    }

    fn field(header: &str) -> Field {
        Field::new(header, ArrowDataType::Utf8, false)
    }
}

impl ArrowAssoc for Option<&str> {
    type Builder = StringBuilder;

    fn builder(nrows: usize) -> Self::Builder {
        StringBuilder::with_capacity(1024, nrows)
    }

    #[throws(ArrowDestinationError)]
    fn append(builder: &mut Self::Builder, value: Self) {
        match value {
            Some(s) => builder.append_value(s),
            None => builder.append_null(),
        }
    }

    fn field(header: &str) -> Field {
        Field::new(header, ArrowDataType::Utf8, true)
    }
}

impl ArrowAssoc for String {
    type Builder = StringBuilder;

    fn builder(nrows: usize) -> Self::Builder {
        StringBuilder::with_capacity(1024, nrows)
    }

    #[throws(ArrowDestinationError)]
    fn append(builder: &mut Self::Builder, value: String) {
        builder.append_value(value.as_str());
    }

    fn field(header: &str) -> Field {
        Field::new(header, ArrowDataType::Utf8, false)
    }
}

impl ArrowAssoc for Option<String> {
    type Builder = StringBuilder;

    fn builder(nrows: usize) -> Self::Builder {
        StringBuilder::with_capacity(1024, nrows)
    }

    #[throws(ArrowDestinationError)]
    fn append(builder: &mut Self::Builder, value: Self) {
        match value {
            Some(s) => builder.append_value(s.as_str()),
            None => builder.append_null(),
        }
    }

    fn field(header: &str) -> Field {
        Field::new(header, ArrowDataType::Utf8, true)
    }
}

impl ArrowAssoc for DateTime<Utc> {
    type Builder = TimestampNanosecondBuilder;

    fn builder(nrows: usize) -> Self::Builder {
        TimestampNanosecondBuilder::with_capacity(nrows)
    }

    #[throws(ArrowDestinationError)]
    fn append(builder: &mut Self::Builder, value: DateTime<Utc>) {
        builder.append_value(
            value
                .timestamp_nanos_opt()
                .unwrap_or_else(|| panic!("out of range DateTime!")),
        )
    }

    fn field(header: &str) -> Field {
        Field::new(
            header,
            ArrowDataType::Timestamp(TimeUnit::Nanosecond, None),
            false,
        )
    }
}

impl ArrowAssoc for Option<DateTime<Utc>> {
    type Builder = TimestampNanosecondBuilder;

    fn builder(nrows: usize) -> Self::Builder {
        TimestampNanosecondBuilder::with_capacity(nrows)
    }

    #[throws(ArrowDestinationError)]
    fn append(builder: &mut Self::Builder, value: Option<DateTime<Utc>>) {
        builder.append_option(value.map(|x| {
            x.timestamp_nanos_opt()
                .unwrap_or_else(|| panic!("out of range DateTime!"))
        }))
    }

    fn field(header: &str) -> Field {
        Field::new(
            header,
            ArrowDataType::Timestamp(TimeUnit::Nanosecond, None),
            true,
        )
    }
}

impl ArrowAssoc for DateTimeWrapperMicro {
    type Builder = TimestampMicrosecondBuilder;

    fn builder(nrows: usize) -> Self::Builder {
        TimestampMicrosecondBuilder::with_capacity(nrows).with_timezone("UTC")
    }

    #[throws(ArrowDestinationError)]
    fn append(builder: &mut Self::Builder, value: DateTimeWrapperMicro) {
        builder.append_value(value.0.timestamp_micros());
    }

    fn field(header: &str) -> Field {
        Field::new(
            header,
            ArrowDataType::Timestamp(TimeUnit::Microsecond, Some("UTC".into())),
            false,
        )
    }
}

impl ArrowAssoc for Option<DateTimeWrapperMicro> {
    type Builder = TimestampMicrosecondBuilder;

    fn builder(nrows: usize) -> Self::Builder {
        TimestampMicrosecondBuilder::with_capacity(nrows).with_timezone("UTC")
    }

    #[throws(ArrowDestinationError)]
    fn append(builder: &mut Self::Builder, value: Option<DateTimeWrapperMicro>) {
        builder.append_option(value.map(|x| x.0.timestamp_micros()));
    }

    fn field(header: &str) -> Field {
        Field::new(
            header,
            ArrowDataType::Timestamp(TimeUnit::Microsecond, Some("UTC".into())),
            true,
        )
    }
}

fn naive_date_to_arrow(nd: NaiveDate) -> i32 {
    match nd.and_hms_opt(0, 0, 0) {
        Some(dt) => (dt.and_utc().timestamp() / SECONDS_IN_DAY) as i32,
        None => panic!("and_hms_opt got None from {:?}", nd),
    }
}

fn naive_datetime_to_arrow(nd: NaiveDateTime) -> i64 {
    nd.and_utc()
        .timestamp_nanos_opt()
        .unwrap_or_else(|| panic!("out of range DateTime"))
}

impl ArrowAssoc for Option<NaiveDate> {
    type Builder = Date32Builder;

    fn builder(nrows: usize) -> Self::Builder {
        Date32Builder::with_capacity(nrows)
    }

    fn append(builder: &mut Self::Builder, value: Option<NaiveDate>) -> Result<()> {
        builder.append_option(value.map(naive_date_to_arrow));
        Ok(())
    }

    fn field(header: &str) -> Field {
        Field::new(header, ArrowDataType::Date32, true)
    }
}

impl ArrowAssoc for NaiveDate {
    type Builder = Date32Builder;

    fn builder(nrows: usize) -> Self::Builder {
        Date32Builder::with_capacity(nrows)
    }

    fn append(builder: &mut Self::Builder, value: NaiveDate) -> Result<()> {
        builder.append_value(naive_date_to_arrow(value));
        Ok(())
    }

    fn field(header: &str) -> Field {
        Field::new(header, ArrowDataType::Date32, false)
    }
}

impl ArrowAssoc for Option<NaiveDateTime> {
    type Builder = TimestampNanosecondBuilder;

    fn builder(nrows: usize) -> Self::Builder {
        TimestampNanosecondBuilder::with_capacity(nrows)
    }

    fn append(builder: &mut Self::Builder, value: Option<NaiveDateTime>) -> Result<()> {
        builder.append_option(value.map(naive_datetime_to_arrow));
        Ok(())
    }

    fn field(header: &str) -> Field {
        Field::new(
            header,
            ArrowDataType::Timestamp(TimeUnit::Nanosecond, None),
            true,
        )
    }
}

impl ArrowAssoc for NaiveDateTime {
    type Builder = TimestampNanosecondBuilder;

    fn builder(nrows: usize) -> Self::Builder {
        TimestampNanosecondBuilder::with_capacity(nrows)
    }

    fn append(builder: &mut Self::Builder, value: NaiveDateTime) -> Result<()> {
        builder.append_value(naive_datetime_to_arrow(value));
        Ok(())
    }

    fn field(header: &str) -> Field {
        Field::new(
            header,
            ArrowDataType::Timestamp(TimeUnit::Nanosecond, None),
            false,
        )
    }
}

impl ArrowAssoc for Option<NaiveDateTimeWrapperMicro> {
    type Builder = TimestampMicrosecondBuilder;

    fn builder(nrows: usize) -> Self::Builder {
        TimestampMicrosecondBuilder::with_capacity(nrows)
    }

    fn append(builder: &mut Self::Builder, value: Option<NaiveDateTimeWrapperMicro>) -> Result<()> {
        builder.append_option(match value {
            Some(v) => Some(v.0.and_utc().timestamp_micros()),
            None => None,
        });
        Ok(())
    }

    fn field(header: &str) -> Field {
        Field::new(
            header,
            ArrowDataType::Timestamp(TimeUnit::Microsecond, None),
            true,
        )
    }
}

impl ArrowAssoc for NaiveDateTimeWrapperMicro {
    type Builder = TimestampMicrosecondBuilder;

    fn builder(nrows: usize) -> Self::Builder {
        TimestampMicrosecondBuilder::with_capacity(nrows)
    }

    fn append(builder: &mut Self::Builder, value: NaiveDateTimeWrapperMicro) -> Result<()> {
        builder.append_value(value.0.and_utc().timestamp_micros());
        Ok(())
    }

    fn field(header: &str) -> Field {
        Field::new(
            header,
            ArrowDataType::Timestamp(TimeUnit::Microsecond, None),
            false,
        )
    }
}

impl ArrowAssoc for Option<NaiveTime> {
    type Builder = Time64NanosecondBuilder;

    fn builder(nrows: usize) -> Self::Builder {
        Time64NanosecondBuilder::with_capacity(nrows)
    }

    fn append(builder: &mut Self::Builder, value: Option<NaiveTime>) -> Result<()> {
        builder.append_option(
            value.map(|t| {
                t.num_seconds_from_midnight() as i64 * 1_000_000_000 + t.nanosecond() as i64
            }),
        );
        Ok(())
    }

    fn field(header: &str) -> Field {
        Field::new(header, ArrowDataType::Time64(TimeUnit::Nanosecond), true)
    }
}

impl ArrowAssoc for NaiveTime {
    type Builder = Time64NanosecondBuilder;

    fn builder(nrows: usize) -> Self::Builder {
        Time64NanosecondBuilder::with_capacity(nrows)
    }

    fn append(builder: &mut Self::Builder, value: NaiveTime) -> Result<()> {
        builder.append_value(
            value.num_seconds_from_midnight() as i64 * 1_000_000_000 + value.nanosecond() as i64,
        );
        Ok(())
    }

    fn field(header: &str) -> Field {
        Field::new(header, ArrowDataType::Time64(TimeUnit::Nanosecond), false)
    }
}

impl ArrowAssoc for Option<NaiveTimeWrapperMicro> {
    type Builder = Time64MicrosecondBuilder;

    fn builder(nrows: usize) -> Self::Builder {
        Time64MicrosecondBuilder::with_capacity(nrows)
    }

    fn append(builder: &mut Self::Builder, value: Option<NaiveTimeWrapperMicro>) -> Result<()> {
        builder.append_option(value.map(|t| {
            t.0.num_seconds_from_midnight() as i64 * 1_000_000 + (t.0.nanosecond() as i64) / 1000
        }));
        Ok(())
    }

    fn field(header: &str) -> Field {
        Field::new(header, ArrowDataType::Time64(TimeUnit::Microsecond), true)
    }
}

impl ArrowAssoc for NaiveTimeWrapperMicro {
    type Builder = Time64MicrosecondBuilder;

    fn builder(nrows: usize) -> Self::Builder {
        Time64MicrosecondBuilder::with_capacity(nrows)
    }

    fn append(builder: &mut Self::Builder, value: NaiveTimeWrapperMicro) -> Result<()> {
        builder.append_value(
            value.0.num_seconds_from_midnight() as i64 * 1_000_000
                + (value.0.nanosecond() as i64) / 1000,
        );
        Ok(())
    }

    fn field(header: &str) -> Field {
        Field::new(header, ArrowDataType::Time64(TimeUnit::Microsecond), false)
    }
}

impl ArrowAssoc for Option<Vec<u8>> {
    type Builder = LargeBinaryBuilder;

    fn builder(nrows: usize) -> Self::Builder {
        LargeBinaryBuilder::with_capacity(1024, nrows)
    }

    fn append(builder: &mut Self::Builder, value: Self) -> Result<()> {
        match value {
            Some(v) => builder.append_value(v),
            None => builder.append_null(),
        };
        Ok(())
    }

    fn field(header: &str) -> Field {
        Field::new(header, ArrowDataType::LargeBinary, true)
    }
}

impl ArrowAssoc for Vec<u8> {
    type Builder = LargeBinaryBuilder;

    fn builder(nrows: usize) -> Self::Builder {
        LargeBinaryBuilder::with_capacity(1024, nrows)
    }

    fn append(builder: &mut Self::Builder, value: Self) -> Result<()> {
        builder.append_value(value);
        Ok(())
    }

    fn field(header: &str) -> Field {
        Field::new(header, ArrowDataType::LargeBinary, false)
    }
}
