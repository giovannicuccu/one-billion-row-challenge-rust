use std::{collections::HashMap, env::args, fs, io::Read, sync::Arc, thread::available_parallelism, usize};

use memmap2::Mmap;

#[derive(Debug, Copy, Clone)]
struct Record {
    count: u32,
    min: i32,
    max: i32,
    sum: i32,
}

impl Record {
    fn default() -> Self {
        Self {
            count: 0,
            min: 1000,
            max: -1000,
            sum: 0,
        }
    }
    fn add(&mut self, value: i32) {
        //println!("add {} cxurr sum={}",value,self.sum);
        self.count += 1;
        self.sum += value;
        self.min = self.min.min(value);
        self.max = self.max.max(value);
    }

    fn merge(&mut self, other: Record) {
        self.count += other.count;
        self.sum += other.sum;
        self.min = self.min.min(other.min);
        self.max = self.max.max(other.max);
    }

    fn avg(&self) -> f32 {
        self.sum as f32 / self.count as f32
    }
}

const DELIMITER_MASK:i128 = 0x3B3B3B3B3B3B3B3B3B3B3B3B3B3B3B3B;
const DELIMITER_MASK_CR:i128 = 0x0A0A0A0A0A0A0A0A0A0A0A0A0A0A0A0A;
const SUB_MASK1:i128 = 0x01010101010101010101010101010101;
const SUB_MASK2:i128 = 0x80808080808080808080808080808080u128 as i128;

const MAP_SIZE:usize=2<<15;
struct CustomMap <'a>{
    element_list: Vec<Option<(usize,usize,i128,Record)>>,
    data: &'a [u8],
}

struct CustomMapIter<'a> {
    current_pos: usize,
    map: &'a CustomMap<'a>,
}

impl <'a> CustomMapIter<'a> {
    pub fn new(map: &'a CustomMap<'a>) -> Self {
        CustomMapIter {current_pos:0, map}
    }

}

impl <'a> Iterator for CustomMapIter<'a> {
    type Item = (&'a[u8],Record);
    fn next(&mut self) -> Option<(&'a[u8],Record)> { 
        while self.current_pos<MAP_SIZE {
            if self.map.element_list[self.current_pos].is_some() {
                let element=self.map.element_list[self.current_pos].as_ref().unwrap();
                let ris= Some((&self.map.data[element.0..element.0+element.1],element.3));
                self.current_pos+=1;
                return ris;
            }
            self.current_pos+=1;
            //println!("next pos in iterator is {}",self.current_pos);
        }
        None
     }
}

const RAW_DATA_MASK:[i128;17] = [0x0, 0xFF, 0xFFFF, 0xFFFF_FF, 
0xFFFF_FFFF,0xFFFF_FFFF_FF,0xFFFF_FFFF_FFFF,0xFFFF_FFFF_FFFF_FF,
0xFFFFFFFFFFFFFFFF,0xFFFFFFFFFFFFFFFFFF,0xFFFFFFFFFFFFFFFFFFFF,0xFFFFFFFFFFFFFFFFFFFFFF,
0xFFFFFFFFFFFFFFFFFFFFFFFF, 0xFFFFFFFFFFFFFFFFFFFFFFFFFF, 0xFFFFFFFFFFFFFFFFFFFFFFFFFFFF, 0xFFFF_FFFF_FFFF_FFFF_FFFF_FFFF_FFFF_FF,
0xFFFF_FFFF_FFFF_FFFF_FFFF_FFFF_FFFF_FFFF_u128 as i128];

impl <'a> CustomMap<'a> {
    pub fn new (data: &'a[u8]) -> Self {
        CustomMap {element_list: vec![None; MAP_SIZE], data}
    }

    pub fn find_index(&mut self, start: usize, len: usize) -> usize {
        let key=&self.data[start..start+len];
        let mut index=self.hash(key)& (MAP_SIZE-1);
        while self.element_list[index].is_some() {
            //println!("there is an entry at {}",index);
            let ref_entry=self.element_list[index].as_ref().unwrap();
            let mapkey=&self.data[ref_entry.0..ref_entry.0+ref_entry.1];
            if mapkey==key {
                return index;
            } else {
                //println!("increment index at {}",index);
                index=(index+1) & (MAP_SIZE-1);
            }
        }
        self.element_list[index]=Some((start,len,0,Record::default()));
        index
    }      

    pub fn find_index_with_raw_data(&mut self, start: usize, len: usize, raw_data: i128) -> usize {
        //let key=&self.data[start..start+len];
        let raw_data_masked=raw_data & RAW_DATA_MASK[len];
        let mut index=self.hash_raw(&raw_data_masked) & (MAP_SIZE-1);
        while self.element_list[index].is_some() {
            //println!("there is an entry at {}",index);
            let ref_entry=self.element_list[index].as_ref().unwrap();
            let map_raw_data=ref_entry.2;
            if raw_data_masked==map_raw_data {
                return index;
            } else {
                //println!("increment index at {}",index);
                index=(index+1) & (MAP_SIZE-1);
            }
        }
        self.element_list[index]=Some((start, len, raw_data_masked, Record::default()));
        index
    } 

    fn add_value(&mut self, index:usize, value:i32 ) {
        //println!("add_value index={}",index);
        self.element_list[index].as_mut().unwrap().3.add(value);
    } 

    fn hash_raw(&self, key: &i128) ->usize {
        fxhash::hash(key)
    }   

    fn hash(&self, key: &[u8]) ->usize {
        fxhash::hash(key)
    }

    fn iter(&self) -> CustomMapIter {
        CustomMapIter::new(&self)
    }
}

impl <'a> IntoIterator for &'a CustomMap<'a> {
    type Item =(&'a[u8],Record);
    type IntoIter = CustomMapIter<'a>;
    fn into_iter(self) -> Self::IntoIter {
        self.iter()       
    }
}


fn convert_into_number(number_word: u64) -> (i32, usize) {
    let decimal_sep_pos = (!number_word & 0x10101000).trailing_zeros();
    let shift = 28 - decimal_sep_pos;
    // signed is -1 if negative, 0 otherwise
    let signed = (!number_word << 59) as i64 >> 63;
    let  design_mask = !(signed & 0xFF);
    // Align the number to a specific position and transform the ascii code
    // to actual digit value in each byte
    let digits = ((number_word & design_mask as u64) << shift) & 0x0F000F0F00;

    // Now digits is in the form 0xUU00TTHH00 (UU: units digit, TT: tens digit, HH: hundreds digit)
    // 0xUU00TTHH00 * (100 * 0x1000000 + 10 * 0x10000 + 1) =
    // 0x000000UU00TTHH00 +
    // 0x00UU00TTHH000000 * 10 +
    // 0xUU00TTHH00000000 * 100
    // Now TT * 100 has 2 trailing zeroes and HH * 100 + TT * 10 + UU < 0x400
    // This results in our value lies in the bit 32 to 41 of this product
    // That was close :)
    let  tmp_value= (digits as u128 * 0x640a0001 as u128) as u128;
    let  abs_value: i32 = (tmp_value >> 32)as i32 & 0x3FF;
    let value: i32 = (abs_value ^ signed as i32) - signed as i32;
    (value,(decimal_sep_pos >> 3) as usize)
}

fn find_in_rawdata(raw_data: i128) -> i128 {
    let comparison_result1 = raw_data ^ DELIMITER_MASK;
    let high_bit_mask1 = (comparison_result1 - SUB_MASK1) & (!comparison_result1 & SUB_MASK2);
    high_bit_mask1
}

fn find_char_in_rawdata(raw_data: i128, delimiter:i128) -> i128 {
    let comparison_result1 = raw_data ^ delimiter;
    let high_bit_mask1 = (comparison_result1 - SUB_MASK1) & (!comparison_result1 & SUB_MASK2);
    high_bit_mask1
}

fn find_slow(data: &[u8], mut offset:usize, mut len:usize) -> (usize,usize) {
    let mut no_content1= true;
    while no_content1 {
        let raw_data=i128::from_le_bytes(data[offset..offset+16].try_into().unwrap());
        let high_bit_mask = find_in_rawdata(raw_data);
        no_content1 = high_bit_mask == 0;
        if no_content1 {
            len+=16;
            offset+=16;
        } else {
            len+= 1+  (high_bit_mask.trailing_zeros()>>3) as usize;
        }
    }
    (offset,len)
}


fn find_next_line(data: &[u8], mut offset:usize, mut len:usize) -> usize {
    let mut no_content1= true;
    while no_content1 {
        let raw_data=i128::from_le_bytes(data[offset..offset+16].try_into().unwrap());
        let high_bit_mask = find_char_in_rawdata(raw_data,DELIMITER_MASK_CR);
        no_content1 = high_bit_mask == 0;
        if no_content1 {
            len+=16;
            offset+=16;
        } else {
            len+= 1+  (high_bit_mask.trailing_zeros()>>3) as usize;
        }
    }
    //println!("len={}",len);
    len
}

fn calculate_local_offset(data: &[u8], begin: usize) -> usize {
    if begin>0 {
        find_next_line(data,begin,0)
    } else {
        0
    }
}

fn calculate_data_slice(data: &[u8], begin: usize, end: usize) -> CustomMap {
    //println!("begin={},end={}",begin, end);
    let mut map=CustomMap::new(&data);
    let mut offset_1=begin;
    let safe=128;
    let step=(end-begin)/3;
    offset_1+=calculate_local_offset(data,offset_1);
    let end_1=begin+step;
    let mut offset_2=end_1;
    offset_2+=calculate_local_offset(data,offset_2);
    let end_2=begin+step*2;
    let mut offset_3=end_2;
    offset_3+=calculate_local_offset(data,offset_3);
    let end_3=end;

    
    let end_1_safe=end_1-safe;
    let end_2_safe=end_2-safe;
    let end_3_safe=end_3-safe;

    //println!("offset_1={},offset_2={},offset_3={}",offset_1, offset_2, offset_3);
    while offset_1<end_1_safe && offset_2<end_2_safe && offset_3<end_3_safe {
        let start_1=offset_1;
        let mut len_1=0;
        
        let start_2=offset_2;
        let mut len_2=0;
        
        let start_3=offset_3;
        let mut len_3=0;

        //let raw_data_1=i128::from_le_bytes(unsafe { *data.get_unchecked(offset_1..).as_ptr().cast() });
        let raw_data_1=i128::from_le_bytes(data[offset_1..offset_1+16].try_into().unwrap());
        let high_bit_mask_1 = find_in_rawdata(raw_data_1);

        //let raw_data_2=i128::from_le_bytes(unsafe { *data.get_unchecked(offset_2..).as_ptr().cast() });
        let raw_data_2=i128::from_le_bytes(data[offset_2..offset_2+16].try_into().unwrap());
        let high_bit_mask_2 = find_in_rawdata(raw_data_2);

        //let raw_data_3=i128::from_le_bytes(unsafe { *data.get_unchecked(offset_3..).as_ptr().cast() });
        let raw_data_3=i128::from_le_bytes(data[offset_3..offset_3+16].try_into().unwrap());
        let high_bit_mask_3 = find_in_rawdata(raw_data_3);
        
        let index_1;
        if high_bit_mask_1 != 0 {
            len_1+= 1+  (high_bit_mask_1.trailing_zeros()>>3) as usize;
            index_1=map.find_index_with_raw_data(start_1, len_1-1, raw_data_1);
        } else {
            (_,len_1)=find_slow(data,offset_1+16,len_1+16);
            //println!("val={}",std::str::from_utf8(&data[start..start+len-1]).unwrap());
            index_1=map.find_index(start_1, len_1-1);
        }

        let index_2;
        if high_bit_mask_2 != 0 {
            len_2+= 1+  (high_bit_mask_2.trailing_zeros()>>3) as usize;
            index_2=map.find_index_with_raw_data(start_2, len_2-1, raw_data_2);
        } else {
            (_,len_2)=find_slow(data,offset_2+16,len_2+16);
            //println!("val={}",std::str::from_utf8(&data[start..start+len-1]).unwrap());
            index_2=map.find_index(start_2, len_2-1);
        }

        let index_3;
        if high_bit_mask_3 != 0 {
            len_3+= 1+  (high_bit_mask_3.trailing_zeros()>>3) as usize;
            index_3=map.find_index_with_raw_data(start_3, len_3-1, raw_data_3);
        } else {
            (_,len_3)=find_slow(data,offset_3+16,len_3+16);
            //println!("val={}",std::str::from_utf8(&data[start..start+len-1]).unwrap());
            index_3=map.find_index(start_3, len_3-1);
        }       

        offset_1=start_1+len_1;
        //let number_word_1=u64::from_le_bytes(unsafe { *data.get_unchecked(offset_1..).as_ptr().cast() });
        let number_word_1=u64::from_le_bytes(data[offset_1..offset_1+8].try_into().unwrap());
        let (value_1, num_offset_1) = convert_into_number(number_word_1);
        map.add_value(index_1, value_1);

        offset_2=start_2+len_2;
        //let number_word_2=u64::from_le_bytes(unsafe { *data.get_unchecked(offset_2..).as_ptr().cast() });
        let number_word_2=u64::from_le_bytes(data[offset_2..offset_2+8].try_into().unwrap());
        let (value_2, num_offset_2) = convert_into_number(number_word_2);
        map.add_value(index_2, value_2);

        offset_3=start_3+len_3;
        //let number_word_3=u64::from_le_bytes(unsafe { *data.get_unchecked(offset_3..).as_ptr().cast() });
        let number_word_3=u64::from_le_bytes(data[offset_3..offset_3+8].try_into().unwrap());
        let (value_3, num_offset_3) = convert_into_number(number_word_3);
        map.add_value(index_3, value_3);

        
        offset_1 += num_offset_1 + 3;
        offset_2 += num_offset_2 + 3;
        offset_3 += num_offset_3 + 3;
        
    }
    //println!("end_1={},end_2={},end_3={}",end_1, end_2, end_3);
    slow_process(data, offset_1, end_1, &mut map);
    slow_process(data, offset_2, end_2, &mut map);
    slow_process(data, offset_3, end_3, &mut map);

    map
}

fn slow_process(data: &[u8], mut offset: usize, end: usize, map: &mut CustomMap) {
    while offset<=end {
        let start=offset;
        let mut len=0;
        while data[offset]!=b';' {
            offset+=1;
            len+=1;
        }
        let index;
        if len<=16 {
            let mut databuf:[u8;16]=[0;16];
            databuf[..len].clone_from_slice(&data[start..start+len]);
            let raw_data_masked=i128::from_le_bytes(databuf);
            index=map.find_index_with_raw_data(start, len, raw_data_masked);
            //map.add_or_update_with_raw_data(start,len, value,raw_data_masked);
        } else {
            index=map.find_index(start, len);
            //map.add_or_update(start,len, value);
        }
        let mut current_position = offset+1;
        let mut value: i32=0;
        let mut sign = 1;
        let b=data[current_position];
        //println!("currch={}",data[current_position] as char);  
        if b == b'-' {
            sign = -1;
        }
        else {
            value = (b - b'0') as i32;
        }
        current_position+=1;
        while data[current_position] != b'.' {    
            //println!("currch={}",data[current_position] as char);        
            value = value * 10 + (data[current_position] - b'0') as i32;
            current_position+=1;
        }
        current_position+=1;
        //println!("currch={}",data[current_position] as char);
        value = value * 10 + (data[current_position] - b'0') as i32;
        if sign == -1 {
            value = -value;
        }
        map.add_value(index, value);

        offset = current_position + 2;

        //println!("station={} val={}",std::str::from_utf8(&data[start..start+len]).unwrap(), value);
    }    
    //println!("slow_offset={}",offset);
}


fn merge<'a>(map:& mut HashMap<Vec<u8>,Record>, custom_map: &'a CustomMap) {
    let v=custom_map.into_iter().collect::<Vec<_>>();
    for (name,record) in v {
        map.entry(name.to_vec()).or_insert(Record::default()).merge(record);
    }
}

fn calculate_data(filename: String) -> String {
    let start: u128 = std::time::SystemTime::now()
    .duration_since(std::time::UNIX_EPOCH)
    .unwrap()
    .as_millis();
    //let mut data_vec = vec![];
    let  mmap: Mmap;
    let  data: &[u8];
    {
        let mut file = std::fs::File::open(filename).unwrap();
        //file.read_to_end(&mut data_vec).unwrap();
        //senza questa riga il programma fallisce
        //assert!(data.pop() == Some(b'\n'));
        mmap = unsafe { Mmap::map(&file).unwrap() };
        data = &*mmap;
    }
    //let data=&data_vec;


    let end: u128 = std::time::SystemTime::now()
    .duration_since(std::time::UNIX_EPOCH)
    .unwrap()
    .as_millis();
    println!("read fs ela={}",end-start);
    
    let num_threads:usize=available_parallelism().unwrap().into();
    println!("num_threads={}",num_threads);
    let step=data.len()/num_threads;
    //let mut idx=0;
    let map:HashMap<Vec<u8>,Record>=HashMap::new();
    let mut sync_map = std::sync::Mutex::new(map);
    // 0  -  1/4
    // 1/4+1 - 1/2
    // 1/2+1 - 3/4
    // 3/4+1 - end
    //let sync_map_arc=Arc::new(&sync_map);
    let mut ranges=vec![];
    for i in 1..num_threads-1 {
        ranges.push(step*i+1..step*(i+1));
    }


    std::thread::scope(|s| {
        //let map_1=sync_map_arc.clone();
        s.spawn( || {
            let ris_map=calculate_data_slice(data,0,step);
            merge(&mut sync_map.lock().unwrap(), &ris_map);
        });
        for range in &ranges {
            s.spawn( || {
            let ris_map=calculate_data_slice(data,range.start,range.end);
            merge(&mut sync_map.lock().unwrap(), &ris_map);
            });
        }
        s.spawn( || {
            let ris_map=calculate_data_slice(data,step*(num_threads-1)+1, data.len()-5);
            merge(&mut sync_map.lock().unwrap(), &ris_map);
        });
        
    });

    /*
    let ris_map=calculate_data_slice(data,0,step);
    merge(&mut sync_map.lock().unwrap(), &ris_map);
    for range in &ranges {
        let ris_map=calculate_data_slice(data,range.start,range.end);
        merge(&mut sync_map.lock().unwrap(), &ris_map);
    }
    let ris_map=calculate_data_slice(data,step*(num_threads-1)+1, data.len()-5);
    merge(&mut sync_map.lock().unwrap(), &ris_map);
    */
    
    //let map_1=calculate_data_slice(data,0,data.len()/2);
    //let map_2=calculate_data_slice(data,data.len()/2+1,data.len()-5);
    //let map_1=calculate_data_slice(data,0,data.len()-5);
    //let mut map = HashMap::new();
    //merge(&mut map, &map_2);
    //merge(&mut map, &map_1);
    //let mut v=map.into_iter().collect::<Vec<_>>();
    
   
    let mut v=sync_map.get_mut().unwrap().into_iter().collect::<Vec<_>>();
    v.sort_unstable_by_key(|p| p.0);
    
    //v.sort_unstable_by_key(|p| p.0.clone());
    let mut result: String = "".to_owned();
    let mut strings=vec![];
    for (name, r) in &v {
        //println!("count for {}={}", std::str::from_utf8(name).unwrap(), r.count);
        strings.push(format!(
            "{}={:.1}/{:.1}/{:.1}",
            //std::str::from_utf8(name).unwrap(),
            std::str::from_utf8(*name).unwrap(),
            r.min as f32/10.0,
            r.avg().round()/10 as f32,
            r.max as f32/10.0
        ));
    }
    result.push_str("{");
    result.push_str(&strings.join(", "));
    result.push_str("}");
    result
}

fn main() {
    let start: u128 = std::time::SystemTime::now()
    .duration_since(std::time::UNIX_EPOCH)
    .unwrap()
    .as_millis();

    //let filename = args().nth(1).unwrap_or("/home/gio/rust/1-billion-row-challenge/measurements.txt".to_string());
    let filename = args().nth(1).unwrap_or("c:\\progetti\\1-billion-row-challenge\\measurements.txt".to_string());
    
    println!("{}",calculate_data(filename));

    let end: u128 = std::time::SystemTime::now()
    .duration_since(std::time::UNIX_EPOCH)
    .unwrap()
    .as_millis();
    println!("ela={}",end-start);
}

#[test]
fn should_calculate_sample() {
    let filename = "c:\\progetti\\1-billion-row-challenge\\measurements_sample.txt".to_string();
    let calculated=calculate_data(filename);
    let expected=fs::read_to_string("C:\\progetti\\my-billion-row-challenge\\app\\src\\test\\resources\\ris_ref_sample.txt").unwrap();
    if calculated!=expected {
        println!("calculated\n{}",calculated.replace(", ", "\n"));
        println!("expected\n{}",expected.replace(", ", "\n"));
    }
    assert_eq!(calculated,expected);
}

/* da indagare meglio 
#[test]
fn test_round() {
    let a=285;
    let b=108;
    let avg=(a+b)as f32/ 2 as f32;
    println!("a+b={}, (a+b)/2={} format (a+b)/2={:1}", a+b, avg/10 as f32, avg.round()/10 as f32);
}
*/