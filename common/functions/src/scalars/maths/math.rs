// Copyright 2021 Datafuse Labs.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use crate::scalars::function_factory::FunctionFactory;
use crate::scalars::AbsFunction;
use crate::scalars::ArithmeticModuloFunction;
use crate::scalars::CRC32Function;
use crate::scalars::CeilFunction;
use crate::scalars::DegressFunction;
use crate::scalars::ExpFunction;
use crate::scalars::FloorFunction;
use crate::scalars::LnFunction;
use crate::scalars::Log10Function;
use crate::scalars::Log2Function;
use crate::scalars::LogFunction;
use crate::scalars::PiFunction;
use crate::scalars::PowFunction;
use crate::scalars::RadiansFunction;
use crate::scalars::RandomFunction;
use crate::scalars::RoundNumberFunction;
use crate::scalars::SignFunction;
use crate::scalars::SqrtFunction;
use crate::scalars::TrigonometricAcosFunction;
use crate::scalars::TrigonometricAsinFunction;
use crate::scalars::TrigonometricAtan2Function;
use crate::scalars::TrigonometricAtanFunction;
use crate::scalars::TrigonometricCosFunction;
use crate::scalars::TrigonometricCotFunction;
use crate::scalars::TrigonometricSinFunction;
use crate::scalars::TrigonometricTanFunction;
use crate::scalars::TruncNumberFunction;

pub struct MathsFunction;

impl MathsFunction {
    pub fn register(factory: &mut FunctionFactory) {
        factory.register("pi", PiFunction::desc());
        factory.register("abs", AbsFunction::desc());
        factory.register("sin", TrigonometricSinFunction::desc());
        factory.register("cos", TrigonometricCosFunction::desc());
        factory.register("tan", TrigonometricTanFunction::desc());
        factory.register("cot", TrigonometricCotFunction::desc());
        factory.register("crc32", CRC32Function::desc());
        factory.register("degrees", DegressFunction::desc());
        factory.register("radians", RadiansFunction::desc());
        factory.register("log", LogFunction::desc());
        factory.register("log10", Log10Function::desc());
        factory.register("log2", Log2Function::desc());
        factory.register("ln", LnFunction::desc());
        factory.register("ceil", CeilFunction::desc());
        factory.register("ceiling", CeilFunction::desc());
        factory.register("floor", FloorFunction::desc());
        factory.register("mod", ArithmeticModuloFunction::desc());
        factory.register("exp", ExpFunction::desc());
        factory.register("asin", TrigonometricAsinFunction::desc());
        factory.register("acos", TrigonometricAcosFunction::desc());
        factory.register("atan", TrigonometricAtanFunction::desc());
        factory.register("atan2", TrigonometricAtan2Function::desc());
        factory.register("sign", SignFunction::desc());
        factory.register("sqrt", SqrtFunction::desc());
        factory.register("pow", PowFunction::desc());
        factory.register("power", PowFunction::desc());
        factory.register("rand", RandomFunction::desc());
        factory.register("round", RoundNumberFunction::desc());
        factory.register("truncate", TruncNumberFunction::desc());
    }
}
