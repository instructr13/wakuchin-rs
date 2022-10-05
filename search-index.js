var searchIndex = JSON.parse('{\
"wakuchin":{"doc":"Core functions of wakuchin tools","t":[0,5,0,0,5,5,0,0,0,5,5,0,3,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,5,5,4,13,11,11,11,11,11,11,11,11,11,11,12,13,3,13,3,13,3,3,4,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,12,11,11,11,11,11,11,11,11,11,11,12,12,12,11,11,11,11,11,11,11,11,11,11,12,12,12,12,12,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,12,12,12,12,3,3,13,4,13,3,11,11,11,11,11,11,11,11,12,12,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,12,12,12,12,12,11,11,11,11,11,11,5,11,11,11,11,11,11,11,12,11,11,11,11,11,11,11,11,11,11,11,11,17,17,17,17,17,17,17,17,17,17,5,5],"n":["builder","check","convert","error","gen","gen_vec","progress","result","symbol","validate","validate_external","worker","ResearchBuilder","borrow","borrow_mut","default","from","into","new","progress_handler","progress_interval","regex","run_par","run_seq","times","tries","try_from","try_into","type_id","workers","chars_to_wakuchin","wakuchin_to_chars","Error","TimesIsZero","borrow","borrow_mut","fmt","fmt","from","into","to_string","try_from","try_into","type_id","0","Done","DoneDetail","Idle","IdleDetail","Processing","ProcessingDetail","Progress","ProgressKind","borrow","borrow","borrow","borrow","borrow","borrow_mut","borrow_mut","borrow_mut","borrow_mut","borrow_mut","clone","clone","clone","clone","clone","clone_into","clone_into","clone_into","clone_into","clone_into","current","fmt","fmt","fmt","fmt","fmt","from","from","from","from","from","id","id","id","into","into","into","into","into","to_owned","to_owned","to_owned","to_owned","to_owned","total","total","total_workers","total_workers","total_workers","try_from","try_from","try_from","try_from","try_from","try_into","try_into","try_into","try_into","try_into","type_id","type_id","type_id","type_id","type_id","wakuchin","0","0","0","Hit","HitCounter","Json","ResultOutputFormat","Text","WakuchinResult","borrow","borrow","borrow","borrow","borrow_mut","borrow_mut","borrow_mut","borrow_mut","chars","chars","clone","clone","clone","clone_into","clone_into","clone_into","deserialize","fmt","fmt","fmt","fmt","from","from","from","from","hit_on","hits","hits","hits_detail","hits_total","into","into","into","into","new","new","out","out","serialize","serialize","serialize","to_owned","to_owned","to_owned","tries","try_from","try_from","try_from","try_from","try_into","try_into","try_into","try_into","type_id","type_id","type_id","type_id","WAKUCHIN","WAKUCHIN_C","WAKUCHIN_EXTERNAL","WAKUCHIN_EXTERNAL_C","WAKUCHIN_EXTERNAL_K","WAKUCHIN_EXTERNAL_N","WAKUCHIN_EXTERNAL_W","WAKUCHIN_K","WAKUCHIN_N","WAKUCHIN_W","run_par","run_seq"],"q":["wakuchin","","","","","","","","","","","","wakuchin::builder","","","","","","","","","","","","","","","","","","wakuchin::convert","","wakuchin::error","","","","","","","","","","","","wakuchin::progress","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","wakuchin::progress::ProgressKind","","","wakuchin::result","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","wakuchin::symbol","","","","","","","","","","wakuchin::worker",""],"d":["","Check wakuchin string with specified regular expression. …","Wakuchin conversion functions","","Generate a randomized wakuchin string.","Generate a vector of randomized wakuchin string. This …","","Functions to manipulate the result of a research","Wakuchin symbol definitions","Check if a string is a internally used wakuchin string.","Check if a string is a wakuchin string.","Wakuchin researcher main functions","","","","","Returns the argument unchanged.","Calls <code>U::from(self)</code>.","","","","","","","","","","","","","Convert from internally used wakuchin chars to actual …","Convert from actual wakuchin chars to internally used …","Error type for wakuchin.","You may specified bad number of times.","","","","","Returns the argument unchanged.","Calls <code>U::from(self)</code>.","","","","","","Worker finished all tasks.","Detail of done progress.","Worker is idle, do nothing.","","Worker is processing something.","Detail of processing progress.","Progress data that you will use in progress_handler.","Kind of progress data.","","","","","","","","","","","","","","","","","","","","","Current processing index.","","","","","","Returns the argument unchanged.","Returns the argument unchanged.","Returns the argument unchanged.","Returns the argument unchanged.","Returns the argument unchanged.","Worker id. 1-indexed, 0 means single worker (sequential).","Worker id. 1-indexed, 0 means single worker (sequential).","Worker id. 1-indexed, 0 means single worker (sequential).","Calls <code>U::from(self)</code>.","Calls <code>U::from(self)</code>.","Calls <code>U::from(self)</code>.","Calls <code>U::from(self)</code>.","Calls <code>U::from(self)</code>.","","","","","","Total number of wakuchin chars to process <em>in this worker</em>.","Total number of wakuchin chars to process <em>in this worker</em>.","Total number of workers.","Total number of workers.","Total number of workers.","","","","","","","","","","","","","","","","Current processing wakuchin chars.","","","","Used when the researcher detects a hit","The count of hits that you will use in progress_handler.","JSON output","The output format of the result","Text output","The result of a research","","","","","","","","","Wakuchin characters that were hit","Wakuchin chars that were hit.","","","","","","","","","","","","Returns the argument unchanged.","Returns the argument unchanged.","Returns the argument unchanged.","Returns the argument unchanged.","The index of the hit","The count of hits.","The count of each hits","A vector of <code>Hit</code>","Total number of hits","Calls <code>U::from(self)</code>.","Calls <code>U::from(self)</code>.","Calls <code>U::from(self)</code>.","Calls <code>U::from(self)</code>.","","Create new hit counter.","Return string of the result with specific output format.","Return string of the result with specific output format. …","","","","","","","The number of tries","","","","","","","","","","","","","Internally used wakuchin chars","Internal wakuchin C","Externally used wakuchin chars","External wakuchin C","External wakuchin K","External wakuchin N","External wakuchin W","Internal wakuchin K","Internal wakuchin N","Internal wakuchin W","Research wakuchin with parallelism.","Research wakuchin with sequential. This function is useful …"],"i":[0,0,0,0,0,0,0,0,0,0,0,0,0,7,7,7,7,7,7,7,7,7,7,7,7,7,7,7,7,7,0,0,0,15,15,15,15,15,15,15,15,15,15,15,22,18,0,18,0,18,0,0,0,18,19,20,21,22,18,19,20,21,22,18,19,20,21,22,18,19,20,21,22,20,18,19,20,21,22,18,19,20,21,22,19,20,21,18,19,20,21,22,18,19,20,21,22,20,21,19,20,21,18,19,20,21,22,18,19,20,21,22,18,19,20,21,22,20,27,28,29,0,0,23,0,23,0,23,24,25,10,23,24,25,10,24,25,23,24,25,23,24,25,23,23,24,25,10,23,24,25,10,24,25,10,10,10,23,24,25,10,24,25,0,10,24,25,10,23,24,25,10,23,24,25,10,23,24,25,10,23,24,25,10,0,0,0,0,0,0,0,0,0,0,0,0],"f":[0,[[1,2],3],0,0,[4,5],[[4,4],[[6,[5]]]],0,0,0,[1,3],[1,3],0,0,[[]],[[]],[[],7],[[]],[[]],[[],7],[7,7],[[7,8],7],[[7,2],[[7,[2]]]],[[[7,[4,4,2]]],9],[[[7,[4,4,2]]],[[13,[10,[12,[11]]]]]],[[7,4],[[7,[4]]]],[[7,4],[[7,[4]]]],[[],13],[[],13],[[],14],[[7,4],7],[1,5],[1,5],0,0,[[]],[[]],[[15,16],17],[[15,16],17],[[]],[[]],[[],5],[[],13],[[],13],[[],14],0,0,0,0,0,0,0,0,0,[[]],[[]],[[]],[[]],[[]],[[]],[[]],[[]],[[]],[[]],[18,18],[19,19],[20,20],[21,21],[22,22],[[]],[[]],[[]],[[]],[[]],0,[[18,16],17],[[19,16],17],[[20,16],17],[[21,16],17],[[22,16],17],[[]],[[]],[[]],[[]],[[]],0,0,0,[[]],[[]],[[]],[[]],[[]],[[]],[[]],[[]],[[]],[[]],0,0,0,0,0,[[],13],[[],13],[[],13],[[],13],[[],13],[[],13],[[],13],[[],13],[[],13],[[],13],[[],14],[[],14],[[],14],[[],14],[[],14],0,0,0,0,0,0,0,0,0,0,[[]],[[]],[[]],[[]],[[]],[[]],[[]],[[]],0,0,[23,23],[24,24],[25,25],[[]],[[]],[[]],[[],[[13,[23]]]],[[23,16],17],[[24,16],17],[[25,16],17],[[10,16],17],[[]],[[]],[[]],[[]],0,0,0,0,0,[[]],[[]],[[]],[[]],[[4,[26,[5]]],24],[[[26,[5]],4],25],[[23,10],[[13,[5,[12,[11]]]]]],[[10,23],[[13,[5,[12,[11]]]]]],[24,13],[25,13],[10,13],[[]],[[]],[[]],0,[[],13],[[],13],[[],13],[[],13],[[],13],[[],13],[[],13],[[],13],[[],14],[[],14],[[],14],[[],14],0,0,0,0,0,0,0,0,0,0,[[4,4,2,8,4],9],[[4,4,2,8],[[13,[10,[12,[11]]]]]]],"p":[[15,"str"],[3,"Regex"],[15,"bool"],[15,"usize"],[3,"String"],[3,"Vec"],[3,"ResearchBuilder"],[3,"Duration"],[8,"Future"],[3,"WakuchinResult"],[8,"Error"],[3,"Box"],[4,"Result"],[3,"TypeId"],[4,"Error"],[3,"Formatter"],[6,"Result"],[4,"ProgressKind"],[3,"IdleDetail"],[3,"ProcessingDetail"],[3,"DoneDetail"],[3,"Progress"],[4,"ResultOutputFormat"],[3,"Hit"],[3,"HitCounter"],[8,"Into"],[13,"Idle"],[13,"Processing"],[13,"Done"]]}\
}');
if (typeof window !== 'undefined' && window.initSearch) {window.initSearch(searchIndex)};
if (typeof exports !== 'undefined') {exports.searchIndex = searchIndex};
